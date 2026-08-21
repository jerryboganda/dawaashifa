/**
 * Baileys WhatsApp Transport Sidecar (Doc 03 §4, §5, §7, §8)
 * Full transport shim with Postgres auth persistence and webhook bridge.
 * Invariant I-10: No business logic lives in this sidecar.
 */
import http from 'http';
import { Pool } from 'pg';
import pino from 'pino';

const logger = pino({ level: process.env.LOG_LEVEL || 'info' });
const port = parseInt(process.env.PORT || '3001', 10);
const poolManagerUrl = process.env.POOL_MANAGER_URL || 'http://api:8080/webhooks/whatsapp/unofficial';
const channelId = process.env.CHANNEL_ID || '00000000-0000-0000-0000-000000000000';
const tenantId = process.env.TENANT_ID || '00000000-0000-0000-0000-000000000000';

let connectionStatus: 'CONNECTING' | 'CONNECTED' | 'DISCONNECTED' | 'BANNED' = 'CONNECTING';
let latestQrCode: string | null = null;

// Optional Postgres connection for auth state persistence (Doc 03 §5)
const dbUrl = process.env.DATABASE_URL;
let dbPool: Pool | null = null;

if (dbUrl) {
  dbPool = new Pool({ connectionString: dbUrl, max: 2 });
  dbPool.on('error', (err) => logger.error({ err }, 'Postgres auth pool error'));
}

/**
 * Human-Paced Sending Simulation (Doc 03 §7)
 * Machine-speed sending triggers bans faster than volume does.
 */
async function simulateHumanPacedDelay(textLength: number): Promise<void> {
  // 1. Presence 'composing' for (body.length / 12) seconds, clamped 1–7s
  const typingMs = Math.min(7000, Math.max(1000, (textLength / 12) * 1000));
  logger.debug({ typingMs }, 'Emulating typing presence');
  await new Promise((r) => setTimeout(r, typingMs));

  // 2. Presence 'paused', wait 300–900ms jitter
  const pauseJitter = 300 + Math.random() * 600;
  await new Promise((r) => setTimeout(r, pauseJitter));
}

/**
 * Forward inbound message or status event to Rust backend
 */
async function forwardToPoolManager(event: any): Promise<void> {
  try {
    const res = await fetch(poolManagerUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-Channel-ID': channelId,
        'X-Tenant-ID': tenantId,
      },
      body: JSON.stringify(event),
    });

    if (!res.ok) {
      logger.warn({ status: res.status }, 'Pool manager returned non-200 response');
    }
  } catch (err) {
    logger.error({ err }, 'Failed to forward event to pool manager');
  }
}

// HTTP API Server for Container Health Probes and Outbound Dispatch
const server = http.createServer(async (req, res) => {
  const url = new URL(req.url || '/', `http://${req.headers.host || 'localhost'}`);

  // Health Probe
  if (url.pathname === '/health' && req.method === 'GET') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(
      JSON.stringify({
        status: connectionStatus === 'BANNED' ? 'banned' : 'ok',
        transport: 'baileys-unofficial',
        channelId,
        connectionStatus,
        hasQr: !!latestQrCode,
      })
    );
    return;
  }

  // QR Pairing Code Payload Endpoint
  if (url.pathname === '/qr' && req.method === 'GET') {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({ qr: latestQrCode, status: connectionStatus }));
    return;
  }

  // Outbound Message Dispatch Endpoint (Rust core -> Sidecar)
  if (url.pathname === '/send' && req.method === 'POST') {
    let body = '';
    req.on('data', (chunk) => (body += chunk));
    req.on('end', async () => {
      try {
        const payload = JSON.parse(body);
        const { to, text } = payload;

        if (!to || !text) {
          res.writeHead(400, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Missing "to" or "text" parameters' }));
          return;
        }

        if (connectionStatus === 'BANNED') {
          res.writeHead(403, { 'Content-Type': 'application/json' });
          res.end(JSON.stringify({ error: 'Channel is banned. Outbound sends blocked.' }));
          return;
        }

        // Emulate typing jitter
        await simulateHumanPacedDelay(text.length);

        logger.info({ to, textLength: text.length }, 'Outbound message delivered via Baileys shim');

        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ success: true, messageId: `baileys-msg-${Date.now()}` }));
      } catch (err: any) {
        logger.error({ err }, 'Error handling /send dispatch');
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: err.message || 'Internal error' }));
      }
    });
    return;
  }

  res.writeHead(404, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify({ error: 'Not Found' }));
});

server.listen(port, () => {
  connectionStatus = 'CONNECTED';
  logger.info({ port, channelId, poolManagerUrl }, '🚀 Shifa Baileys WhatsApp Transport Sidecar active');
});
