/**
 * PAGI XCURZENS - Architect View
 * Studio UI runs on port 3001. Gateway at 127.0.0.1:8000.
 */
import { GATEWAY_ORIGIN } from './api/config';

const root = document.getElementById('root');
if (root) {
  root.innerHTML = `<h1>PAGI XCURZENS</h1><p>Architect View · Gateway: ${GATEWAY_ORIGIN}</p>`;
}
