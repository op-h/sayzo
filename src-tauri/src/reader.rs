use crate::history::History;
use crate::settings::SettingsState;
use crate::text::{self, Sentence};
use crate::tts::{Engines, OUT_SR};
use anyhow::{anyhow, Result};
use rodio::buffer::SamplesBuffer;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{OutputStream, OutputStreamHandle, Sink};
use serde::Serialize;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager};

/// How many synthesized sentences may wait in the audio queue. Enough to hide
/// generation time, small enough that long documents don't fill memory.
const LOOKAHEAD: usize = 3;
/// Short pause inserted when one sentence switches between Arabic and English voices
const SEGMENT_GAP_MS: usize = 90;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Idle,
    Loading,
    Speaking,
    Paused,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub status: Status,
    pub current: usize,
    pub total: usize,
    pub error: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub sentences: Vec<Sentence>,
    pub source: String,
    #[serde(flatten)]
    pub progress: Progress,
}

struct State {
    status: Status,
    sentences: Vec<Sentence>,
    source: String,
    current: usize,
    /// Sentence index of every buffer appended to the current sink, in order
    queued: VecDeque<usize>,
    generating: bool,
    error: Option<String>,
    voice_override: Option<String>,
    sink: Option<Arc<Sink>>,
}

/// The audio device stream must stay on the thread that opened it, so it
/// lives on its own thread until `_stop` is dropped
struct Output {
    handle: OutputStreamHandle,
    device: String,
    _stop: mpsc::Sender<()>,
}

#[derive(Clone)]
pub struct Reader {
    app: AppHandle,
    engines: Arc<Engines>,
    session: Arc<AtomicU64>,
    state: Arc<Mutex<State>>,
    output: Arc<Mutex<Option<Output>>>,
}

fn default_device_name() -> String {
    rodio::cpal::default_host()
        .default_output_device()
        .and_then(|d| d.name().ok())
        .unwrap_or_default()
}

fn open_output() -> Result<Output> {
    let (tx, rx) = mpsc::channel();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    thread::spawn(move || match OutputStream::try_default() {
        Ok((stream, handle)) => {
            let _ = tx.send(Ok(handle));
            let _ = stop_rx.recv();
            drop(stream);
        }
        Err(e) => {
            let _ = tx.send(Err(anyhow!("no audio output device: {e}")));
        }
    });
    let handle = rx.recv()??;
    Ok(Output { handle, device: default_device_name(), _stop: stop_tx })
}

impl Reader {
    pub fn new(app: AppHandle, engines: Arc<Engines>) -> Self {
        let reader = Self {
            app,
            engines,
            session: Arc::new(AtomicU64::new(0)),
            state: Arc::new(Mutex::new(State {
                status: Status::Idle,
                sentences: Vec::new(),
                source: String::new(),
                current: 0,
                queued: VecDeque::new(),
                generating: false,
                error: None,
                voice_override: None,
                sink: None,
            })),
            output: Arc::new(Mutex::new(None)),
        };
        reader.spawn_monitor();
        reader
    }

    /// Reopens the audio stream when Windows' default output changed (for
    /// example headphones were plugged in) so speech follows the user's device
    fn new_sink(&self) -> Result<Arc<Sink>> {
        let mut out = self.output.lock().unwrap();
        let current = default_device_name();
        if out.as_ref().map(|o| o.device != current).unwrap_or(true) {
            *out = Some(open_output()?);
        }
        let sink = Sink::try_new(&out.as_ref().unwrap().handle)?;
        sink.set_volume(self.app.state::<SettingsState>().get().volume);
        Ok(Arc::new(sink))
    }

    pub fn speak(&self, raw: &str, source: &str, voice_override: Option<String>) -> Result<()> {
        let sentences = text::split(raw);
        if sentences.is_empty() {
            return Ok(());
        }
        if voice_override.is_none() {
            self.app.state::<History>().add(raw.trim(), source);
        }
        self.start(sentences, 0, source.to_string(), voice_override)
    }

    fn start(&self, sentences: Vec<Sentence>, from: usize, source: String, voice_override: Option<String>) -> Result<()> {
        let sink = self.new_sink()?;
        let id = {
            let mut st = self.state.lock().unwrap();
            // Bumping the session under the state lock guarantees an older
            // worker can never append audio after this point
            let id = self.session.fetch_add(1, Ordering::SeqCst) + 1;
            st.sink = Some(sink.clone());
            st.status = Status::Loading;
            st.sentences = sentences.clone();
            st.source = source;
            st.current = from;
            st.queued.clear();
            st.generating = true;
            st.error = None;
            st.voice_override = voice_override.clone();
            id
        };
        self.emit_session();

        let this = self.clone();
        thread::spawn(move || this.generate(id, sink, sentences, from, voice_override));
        Ok(())
    }

    fn generate(&self, id: u64, sink: Arc<Sink>, sentences: Vec<Sentence>, from: usize, voice_override: Option<String>) {
        let alive = {
            let session = self.session.clone();
            move || session.load(Ordering::SeqCst) == id
        };
        for (idx, sentence) in sentences.iter().enumerate().skip(from) {
            while alive() && sink.len() >= LOOKAHEAD {
                thread::sleep(Duration::from_millis(40));
            }
            if !alive() {
                return;
            }
            // Settings are read per sentence so speed and voice changes apply mid-read
            let s = self.app.state::<SettingsState>().get();
            let (en, ar) = match &voice_override {
                Some(v) if v.starts_with("ar-") => (s.english_voice.clone(), v.clone()),
                Some(v) => (v.clone(), s.arabic_voice.clone()),
                None => (s.english_voice.clone(), s.arabic_voice.clone()),
            };
            let mut samples = Vec::new();
            for (i, (lang, seg)) in sentence.segments.iter().enumerate() {
                if i > 0 {
                    samples.extend(std::iter::repeat(0.0).take(OUT_SR as usize * SEGMENT_GAP_MS / 1000));
                }
                match self.engines.synth(*lang, seg, &en, &ar, s.speed, alive.clone()) {
                    Ok(Some(mut audio)) => samples.append(&mut audio),
                    Ok(None) => return,
                    Err(e) => {
                        self.fail(id, e.to_string());
                        return;
                    }
                }
            }
            let mut st = self.state.lock().unwrap();
            if !alive() {
                return;
            }
            st.queued.push_back(idx);
            if st.status == Status::Loading {
                st.status = Status::Speaking;
            }
            sink.append(SamplesBuffer::new(1, OUT_SR, samples));
        }
        let mut st = self.state.lock().unwrap();
        if alive() {
            st.generating = false;
        }
    }

    fn fail(&self, id: u64, error: String) {
        let mut st = self.state.lock().unwrap();
        if self.session.load(Ordering::SeqCst) != id {
            return;
        }
        st.generating = false;
        st.error = Some(error);
        if st.queued.is_empty() {
            st.status = Status::Idle;
        }
        drop(st);
        self.emit_progress();
    }

    /// Follows playback to know which sentence is audible and when reading ends
    fn spawn_monitor(&self) {
        let this = self.clone();
        thread::spawn(move || {
            let mut last: Option<(Status, usize)> = None;
            loop {
                thread::sleep(Duration::from_millis(50));
                let snapshot = {
                    let mut st = this.state.lock().unwrap();
                    if let Some(sink) = st.sink.clone() {
                        let finished = st.queued.len().saturating_sub(sink.len());
                        if let Some(&idx) = st.queued.get(finished) {
                            st.current = idx;
                        }
                        if !st.generating && sink.empty() && matches!(st.status, Status::Speaking | Status::Loading) {
                            st.status = Status::Idle;
                            st.sink = None;
                        }
                    }
                    (st.status, st.current)
                };
                if last != Some(snapshot) {
                    last = Some(snapshot);
                    this.emit_progress();
                }
            }
        });
    }

    pub fn pause(&self) {
        let mut st = self.state.lock().unwrap();
        if let (Some(sink), Status::Speaking | Status::Loading) = (&st.sink, st.status) {
            sink.pause();
            st.status = Status::Paused;
        }
    }

    pub fn resume(&self) {
        let mut st = self.state.lock().unwrap();
        if let (Some(sink), Status::Paused) = (&st.sink, st.status) {
            sink.play();
            st.status = if st.queued.is_empty() { Status::Loading } else { Status::Speaking };
        }
    }

    pub fn toggle_pause(&self) {
        let status = self.state.lock().unwrap().status;
        match status {
            Status::Paused => self.resume(),
            Status::Speaking | Status::Loading => self.pause(),
            Status::Idle => {}
        }
        self.emit_progress();
    }

    pub fn is_active(&self) -> bool {
        self.state.lock().unwrap().status != Status::Idle
    }

    pub fn stop(&self) {
        let mut st = self.state.lock().unwrap();
        self.session.fetch_add(1, Ordering::SeqCst);
        st.sink = None;
        st.status = Status::Idle;
        st.generating = false;
        st.queued.clear();
        drop(st);
        self.emit_progress();
    }

    /// Restarts reading `delta` sentences away from the one currently audible
    pub fn jump(&self, delta: i64) -> Result<()> {
        let (sentences, target, source, voice) = {
            let st = self.state.lock().unwrap();
            if st.sentences.is_empty() {
                return Ok(());
            }
            let target = (st.current as i64 + delta).clamp(0, st.sentences.len() as i64 - 1) as usize;
            (st.sentences.clone(), target, st.source.clone(), st.voice_override.clone())
        };
        self.start(sentences, target, source, voice)
    }

    /// Like `jump` but to an absolute sentence, used when clicking a sentence in the panel
    pub fn seek(&self, index: usize) -> Result<()> {
        let current = self.state.lock().unwrap().current as i64;
        self.jump(index as i64 - current)
    }

    pub fn set_volume(&self, volume: f32) {
        if let Some(sink) = &self.state.lock().unwrap().sink {
            sink.set_volume(volume);
        }
    }

    fn progress(st: &State) -> Progress {
        Progress { status: st.status, current: st.current, total: st.sentences.len(), error: st.error.clone() }
    }

    pub fn snapshot(&self) -> Snapshot {
        let st = self.state.lock().unwrap();
        Snapshot { sentences: st.sentences.clone(), source: st.source.clone(), progress: Self::progress(&st) }
    }

    fn emit_session(&self) {
        let _ = self.app.emit("reader://session", self.snapshot());
    }

    fn emit_progress(&self) {
        let p = Self::progress(&self.state.lock().unwrap());
        let _ = self.app.emit("reader://progress", p);
    }

    /// Renders the whole text to a WAV file in the user's Music\Sayzo folder
    pub fn export(&self, raw: &str) -> Result<PathBuf> {
        let sentences = text::split(raw);
        if sentences.is_empty() {
            return Err(anyhow!("nothing to export"));
        }
        let s = self.app.state::<SettingsState>().get();
        let dir = self.app.path().audio_dir()?.join("Sayzo");
        std::fs::create_dir_all(&dir)?;
        let title: String = sentences[0]
            .text
            .chars()
            .filter(|c| c.is_alphanumeric() || *c == ' ')
            .take(40)
            .collect::<String>()
            .trim()
            .to_string();
        let title = if title.is_empty() { "speech".to_string() } else { title };
        let mut path = dir.join(format!("{title}.wav"));
        let mut n = 2;
        while path.exists() {
            path = dir.join(format!("{title} ({n}).wav"));
            n += 1;
        }
        let spec = hound::WavSpec { channels: 1, sample_rate: OUT_SR, bits_per_sample: 16, sample_format: hound::SampleFormat::Int };
        let mut writer = hound::WavWriter::create(&path, spec)?;
        let pause = vec![0.0f32; OUT_SR as usize / 4];
        for sentence in &sentences {
            for (lang, seg) in &sentence.segments {
                if let Some(audio) = self.engines.synth(*lang, seg, &s.english_voice, &s.arabic_voice, s.speed, || true)? {
                    for x in audio.iter().chain(pause.iter()) {
                        writer.write_sample((x.clamp(-1.0, 1.0) * i16::MAX as f32) as i16)?;
                    }
                }
            }
        }
        writer.finalize()?;
        Ok(path)
    }
}
