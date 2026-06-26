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
          <div className="h-[72vh] rounded-2xl border border-slate-800 bg-slate-900/70" />
          <div className="space-y-4">
            <div className="h-28 rounded-2xl border border-slate-800 bg-slate-900/70" />
            <div className="h-56 rounded-2xl border border-slate-800 bg-slate-900/70" />
            <div className="grid gap-4 xl:grid-cols-2">
              <div className="h-40 rounded-2xl border border-slate-800 bg-slate-900/70" />
              <div className="h-40 rounded-2xl border border-slate-800 bg-slate-900/70" />
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
