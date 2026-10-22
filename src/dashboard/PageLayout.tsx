import { Header, ScrollArea } from "@/components";

export const PageLayout = ({
  title,
  description,
  rightSlot,
  children,
}: {
  title: string;
  description: string;
  rightSlot?: React.ReactNode;
  children: React.ReactNode;
}) => (
  <div className="flex flex-1 flex-col min-h-0">
    <header className="pt-8 pl-1 pr-7">
      <Header isMainTitle title={title} description={description} rightSlot={rightSlot} />
    </header>
    {/* Fills whatever height the header leaves, so the last item is always reachable */}
    <ScrollArea className="flex-1 min-h-0 pr-6">
      <div className="flex flex-col gap-6 pb-12 pt-4 px-1">{children}</div>
    </ScrollArea>
  </div>
);
