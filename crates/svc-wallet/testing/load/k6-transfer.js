import http from 'k6/http';
import { sleep } from 'k6';
export const options = { vus: 10, duration: '10s' };
export default function () {
  http.post('http://127.0.0.1:8080/v1/transfer', JSON.stringify({}), {
    headers: { 'Content-Type': 'application/json', 'Idempotency-Key': 'demo' }
  });
  sleep(0.1);
}
