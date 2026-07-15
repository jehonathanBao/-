export default function PageShellSkeleton() {
  return (
    <div
      className="min-h-screen bg-slate-950 px-4 py-6 text-slate-100 md:px-6 lg:px-8"
      data-testid="page-shell-skeleton"
    >
      <div className="mx-auto max-w-7xl space-y-4">
        <p className="text-xs uppercase tracking-[0.3em] text-cyan-300">Loading dashboard</p>
        <div className="h-10 w-64 rounded-xl bg-slate-800/70" />
        <div className="grid gap-4 lg:grid-cols-[260px_minmax(0,1fr)]">
          <div className="workspace-panel h-[72vh]" />
          <div className="space-y-4">
            <div className="workspace-panel h-28" />
            <div className="workspace-panel h-56" />
            <div className="grid gap-4 xl:grid-cols-2">
              <div className="workspace-panel h-40" />
              <div className="workspace-panel h-40" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
