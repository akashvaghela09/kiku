/**
 * The band at the top of every screen.
 *
 * The rail says where you can go; this says where you are and carries whatever acts on
 * what is below it. It lives in the shell rather than in `components/ui` because it is
 * one fixed part of the frame, not a primitive anyone should be composing with.
 *
 * Both screens render it, and both get the same 56px band whether or not they have
 * anything to put in it - previously History had a header and Settings had none, so
 * their scroll regions began at different heights.
 */
export interface ViewToolbarProps {
  title: string;
  /** Muted text beside the title: a count, or where you are within the screen. */
  meta?: React.ReactNode | undefined;
  /** Actions, aligned to the trailing edge. */
  children?: React.ReactNode | undefined;
}

export function ViewToolbar({ title, meta, children }: ViewToolbarProps) {
  return (
    <header className="shrink-0 border-b border-border-subtle bg-surface-sunken">
      <div className="app-column flex h-14 items-center gap-3">
        <div className="flex min-w-0 flex-1 items-center gap-2.5">
          <h1 className="shrink-0 text-lg font-semibold text-primary">{title}</h1>
          {meta && <span className="truncate text-ui text-muted">{meta}</span>}
        </div>
        {children}
      </div>
    </header>
  );
}
