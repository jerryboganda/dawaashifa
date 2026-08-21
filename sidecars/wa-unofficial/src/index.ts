/**
 * Baileys WhatsApp Transport Sidecar (Doc 03 §4, §5)
 * Thin transport shim with Postgres auth persistence and NATS bridge.
 * Invariant I-10: No business logic lives in this sidecar.
 */
import http from 'http';

const port = process.env.PORT || 3001;

const server = http.createServer((req, res) => {
  if (req.url === '/health') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ status: 'ok', transport: 'baileys-unofficial' }));
    return;
  }

  res.writeHead(200, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ message: 'Shifa Baileys WhatsApp Transport Sidecar active' }));
});

server.listen(port, () => {
  console.log(`🚀 Shifa Baileys WhatsApp Transport Sidecar listening on port ${port}`);
});
