export default function WorkspacePageHeader({ eyebrow, title, description, badge }) {
  return (
    <section className="workspace-page-header" data-testid="workspace-page-header">
      <div className="workspace-page-heading">
        <p>{eyebrow}</p>
        <h2>{title}</h2>
        <span>{description}</span>
      </div>
      {badge ? <div className="workspace-page-badge">{badge}</div> : null}
    </section>
  );
}
