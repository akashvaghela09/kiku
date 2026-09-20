import { Row } from '@/components/ui';
import { SETTINGS_SECTIONS } from '@/features/settings/sections';
import { cn } from '@/lib/cn';
import type { EngineStatus } from '@/lib/ipc';
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
}

export function NavigationRail({
  view,
  onNavigate,
  engine,
  activeSection,
  onSection,
}: NavigationRailProps) {
  return (
    <nav
      aria-label="Main"
      className="flex w-[200px] shrink-0 flex-col border-r border-border-subtle bg-surface"
    >
      <div className="px-4 pb-3 pt-4">
        <span className="text-lg font-semibold tracking-tight text-primary">Kiku</span>
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

      <EngineIndicator engine={engine} onFix={() => onNavigate('settings')} />
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

  return (
    <div className="border-t border-border-subtle px-4 py-2.5">
      {failed ? (
        <button type="button" onClick={onFix} className="w-full text-left hover:underline">
          {body}
        </button>
      ) : (
        body
      )}
    </div>
  );
}
