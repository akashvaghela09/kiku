/**
 * The complete set of interface primitives.
 *
 * Fourteen components, and the bar for a fifteenth is that it appears on at least two
 * surfaces or is the signature of one. Anything assembled from these lives in
 * `features/`, not here.
 */
export { Badge } from './Badge';
export { Button } from './Button';
export { Dialog } from './Dialog';
export { EmptyState } from './EmptyState';
export { Input } from './Input';
export { Kbd } from './Kbd';
export { Panel } from './Panel';
export { Progress } from './Progress';
export { Row } from './Row';
export { Select } from './Select';
export { Slider } from './Slider';
export { Toast } from './Toast';
export { Toggle } from './Toggle';
export { WaveBars } from '@/features/overlay/WaveBars';

export type { BadgeProps } from './Badge';
export type { ButtonProps } from './Button';
export type { DialogProps } from './Dialog';
export type { EmptyStateProps } from './EmptyState';
export type { InputProps } from './Input';
export type { KbdProps } from './Kbd';
export type { PanelProps } from './Panel';
export type { ProgressProps } from './Progress';
export type { RowProps } from './Row';
export type { SelectOption, SelectProps } from './Select';
export type { SliderProps } from './Slider';
export type { ToastProps } from './Toast';
export type { ToggleProps } from './Toggle';
