import { createRoot } from 'react-dom/client';

import '@/styles/global.css';
import { Overlay } from './Overlay';

const container = document.getElementById('root');
if (!container) throw new Error('missing #root');

// No StrictMode here: it double-invokes effects, and the overlay's waveform drives an
// imperative rAF loop where a duplicated loop would double the frame cost.
createRoot(container).render(<Overlay />);
