import { useEffect, useState } from "react";
import { MemoryRouter, Outlet, Route, Routes, useLocation, useNavigate } from "react-router-dom";
import { AudioLinesIcon, GithubIcon, HistoryIcon, HouseIcon, KeyboardIcon, PowerIcon, SettingsIcon } from "lucide-react";
import { api } from "@/lib/api";
import { cn } from "@/lib/utils";
import { Home } from "./Home";
import { Voices } from "./Voices";
import { History } from "./History";
import { Shortcuts } from "./Shortcuts";
import { SettingsPage } from "./Settings";

const MENU = [
  { label: "Home", href: "/", icon: HouseIcon },
  { label: "Voices", href: "/voices", icon: AudioLinesIcon },
  { label: "History", href: "/history", icon: HistoryIcon },
  { label: "Shortcuts", href: "/shortcuts", icon: KeyboardIcon },
  { label: "Settings", href: "/settings", icon: SettingsIcon },
];

const Sidebar = () => {
  const navigate = useNavigate();
  const active = useLocation().pathname;
  const [version, setVersion] = useState("");
  useEffect(() => {
    api.getVersion().then(setVersion);
  }, []);

  const itemClass = (selected: boolean) =>
    cn(
      "flex w-full items-center gap-3 rounded-xl px-3 py-2 text-xs lg:text-sm text-sidebar-foreground/70 transition-all duration-300 hover:bg-sidebar-accent hover:text-sidebar-accent-foreground",
      selected && "font-medium bg-sidebar-accent text-sidebar-accent-foreground"
    );

  return (
    <aside className="flex w-56 flex-col select-none pt-2">
      <div onClick={() => navigate("/")} className="flex h-16 items-center px-4 pt-6 gap-1.5 cursor-pointer">
        <div className="flex size-6 lg:size-7 items-center justify-center rounded-lg bg-primary">
          <AudioLinesIcon className="size-4 lg:size-5 text-primary-foreground" />
        </div>
        <div className="flex flex-col">
          <h1 className="text-xs lg:text-base font-semibold text-foreground">Sayzo</h1>
          <span className="text-[10px] text-muted-foreground leading-none mt-0.5 block">{version ? `v${version}` : ""}</span>
        </div>
      </div>

      <nav className="flex-1 space-y-1 px-3 py-6">
        {MENU.map((item) => (
          <button key={item.href} onClick={() => navigate(item.href)} className={itemClass(active === item.href)}>
            <item.icon className="size-3 lg:size-4" />
            {item.label}
          </button>
        ))}
      </nav>

      <div className="flex flex-col space-y-1 px-3 pb-3">
        <button onClick={() => api.exit()} className={itemClass(false)}>
          <PowerIcon className="size-3 lg:size-4" />
          Quit Sayzo
        </button>
        <button
          onClick={() => api.openAuthorPage()}
          title="github.com/op-h"
          className="flex w-full items-center gap-2 rounded-xl px-3 pt-2 text-[10px] lg:text-xs text-muted-foreground hover:text-foreground transition-colors"
        >
          <GithubIcon className="size-3" />
          Designed by <span className="font-semibold">OPH</span>
        </button>
      </div>
    </aside>
  );
};

const Layout = () => (
  <div className="relative flex h-screen w-screen overflow-hidden bg-background">
    <Sidebar />
    <main className="flex flex-1 flex-col overflow-hidden px-8">
      <Outlet />
    </main>
  </div>
);

export const Dashboard = () => (
  <MemoryRouter>
    <Routes>
      <Route element={<Layout />}>
        <Route path="/" element={<Home />} />
        <Route path="/voices" element={<Voices />} />
        <Route path="/history" element={<History />} />
        <Route path="/shortcuts" element={<Shortcuts />} />
        <Route path="/settings" element={<SettingsPage />} />
      </Route>
    </Routes>
  </MemoryRouter>
);
