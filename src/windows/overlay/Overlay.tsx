/**
 * The capsule — the floating listening indicator.
 *
 * Scaffold only: fixed geometry and static bars, enough to confirm the transparent
 * click-through window renders correctly on each platform. The amplitude-driven
 * waveform and state machine arrive in chunk 6.
 */
const BAR_COUNT = 15;

export function Overlay() {
  return (
    <div className="flex h-full w-full items-center justify-center">
      <div
        className="flex h-11 items-center gap-[5px] rounded-pill px-5"
        style={{
          background: 'var(--ov-bg)',
          boxShadow: 'var(--ov-shadow)',
          border: '1px solid var(--ov-hairline)',
        }}
      >
        {Array.from({ length: BAR_COUNT }, (_, index) => (
          <span
            key={index}
            className="w-1 rounded-pill"
            style={{ height: 16, background: 'var(--ov-bar-idle)' }}
          />
        ))}
      </div>
    </div>
  );
}
