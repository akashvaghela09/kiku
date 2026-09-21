import { Badge, Row } from '@/components/ui';
import { SETTINGS_SECTIONS } from '@/features/settings/sections';
import { cn } from '@/lib/cn';
import { commands, type EngineStatus, type UpdateStatus } from '@/lib/ipc';
import { VIEWS, VIEW_LABELS, type View } from './views';

/**
 * The application's one navigation system.
 *
 * Previously there were two: a bottom bar for the two screens, and a sidebar inside
 * Settings for its sections. That inverted the hierarchy - the most consequential
 * control, which screen you are on, was the smallest and lowest thing in the window,
 * while scrolling to a section got a permanent rail.
 *
 * Here the rail carries both, at two depths. Settings' sections expand beneath it
 * when Settings is open, indented and without icons, because the parent already
 * carries the icon and repeating it is noise.
 */

interface NavigationRailProps {
  view: View;
  onNavigate: (view: View) => void;
  engine: EngineStatus;
  activeSection: string | null;
  onSection: (id: string) => void;
  /** Own version, shown at the foot. */
  version: string | null;
  /** `null` while unknown, when checking is switched off, or when offline. */
  update: UpdateStatus | null;
}

export function NavigationRail({
  view,
  onNavigate,
  engine,
  activeSection,
  onSection,
  version,
  update,
}: NavigationRailProps) {
  const available = update?.state === 'available' ? update.detail : null;
  return (
    <nav
      aria-label="Main"
      className="flex w-[200px] shrink-0 flex-col border-r border-border-subtle bg-surface"
    >
      <div className="flex items-center gap-2 px-4 pb-3 pt-4">
        <span className="text-lg font-semibold tracking-tight text-primary">Kiku</span>

        {/*
         * Beside the name, because that is where you look to find out what this is,
         * and an update is a fact about the application rather than about any screen
         * in it. Solid rather than soft: the soft accent badge is painted in
         * `--accent-wash`, which is the same value as `--surface-selected`, the colour
         * the rows below use to mean "you are here".
         *
         * It opens the release page. Kiku cannot install anything, so the label says
         * Update rather than promising the act of updating.
         */}
        {available && (
          <button
            type="button"
            onClick={() => void commands.openUrl(available.url)}
            title={`Version ${available.version} is available`}
            aria-label={`Version ${available.version} is available. Opens the release page.`}
            className="rounded-sm transition-opacity duration-100 hover:opacity-85"
          >
            <Badge tone="accent" variant="solid">
              Update
            </Badge>
          </button>
        )}
      </div>

      <div className="min-h-0 flex-1 overflow-auto px-2 pb-2">
        {VIEWS.map(({ id, icon: Icon }) => (
          <Row
            key={id}
            as="li"
            density="compact"
            interactive
            selected={view === id}
            onActivate={() => onNavigate(id)}
            leading={<Icon size={15} aria-hidden />}
            title={VIEW_LABELS[id]}
            className="list-none"
          />
        ))}

        {/* Sections belong to Settings, so they only exist while Settings does. */}
        {view === 'settings' && (
          <ul className="mt-0.5 space-y-px">
            {SETTINGS_SECTIONS.map((section) => (
              <li key={section.id}>
                <button
                  type="button"
                  onClick={() => onSection(section.id)}
                  className={cn(
                    'w-full rounded-md py-1 pl-[34px] pr-2 text-left text-ui',
                    'transition-colors duration-100 hover:bg-surface-hover',
                    activeSection === section.id
                      ? 'text-accent-text'
                      : 'text-secondary hover:text-primary',
                  )}
                  aria-current={activeSection === section.id ? 'true' : undefined}
                >
                  {section.label}
                </button>
              </li>
            ))}
          </ul>
        )}
      </div>

      {/*
       * One strip, not two stacked blocks: the engine message and the version are the
       * same kind of thing, quiet status at the foot in the same register, so they
       * share a container and a type scale.
       *
       * No rule above it. There is nothing on the other side of a line here - the list
       * above simply ends - so a border would be drawing a box for its own sake. The
       * padding is doing the separating.
       */}
      {(engine.state !== 'ready' || version) && (
        <div className="px-4 pb-2.5 pt-3">
          <EngineIndicator engine={engine} onFix={() => onNavigate('settings')} />

          {version && (
            <div className={cn(engine.state !== 'ready' && 'mt-2')}>
              {/* Selectable because its whole job is being pasted into a bug report,
                  and the rail is otherwise `user-select: none`. */}
              <span className="select-text text-xs text-muted">Kiku v{version}</span>

            </div>
          )}
        </div>
      )}
    </nav>
  );
}

/**
 * Engine state, pinned to the bottom of the rail.
 *
 * A failure you cannot act on is worse than no message, so a failed engine is
 * clickable and takes you to the model settings.
 */
function EngineIndicator({
  engine,
  onFix,
}: {
  engine: EngineStatus;
  onFix: () => void;
}) {
  if (engine.state === 'ready') return null;

  const failed = engine.state === 'failed';
  const label =
    engine.state === 'loading'
      ? 'Getting the model ready'
      : failed
        ? 'The model could not load'
        : 'No model installed';

  const dot = (
    <span
      aria-hidden
      className={cn(
        'size-1.5 shrink-0 rounded-full',
        failed ? 'bg-danger' : 'bg-accent',
        engine.state === 'loading' && 'animate-pulse',
      )}
    />
  );

  const body = (
    <span className="flex items-center gap-2 text-xs">
      {dot}
      <span className={cn('truncate', failed ? 'text-danger' : 'text-muted')}>{label}</span>
    </span>
  );

  // No wrapper of its own: the foot supplies the border and the padding, so this and
  // the version line sit in one strip rather than two.
  return failed ? (
    <button type="button" onClick={onFix} className="w-full text-left hover:underline">
      {body}
    </button>
  ) : (
    body
  );
}
