/**
 * Join class names, dropping anything falsy.
 *
 * Three lines instead of a dependency. There is no Tailwind class-conflict resolution
 * here on purpose: components own their classes, and callers override through props
 * rather than by passing competing utilities.
 */
export function cn(...parts: (string | false | null | undefined)[]): string {
  return parts.filter(Boolean).join(' ');
}
