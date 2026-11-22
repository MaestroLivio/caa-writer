#!/usr/bin/env node
// Client CDP minimale per Recordly (Electron): valuta un'espressione JS nella pagina
// indicata e stampa il risultato. Usa il WebSocket nativo di Node (>= 22).
//
//   node tutorial/cdp.js '<espressione>'            # prima pagina disponibile
//   CDP_FILTER=editor node tutorial/cdp.js '<expr>' # pagina il cui url contiene il filtro
//   CDP_TIMEOUT=20000 node tutorial/cdp.js '<expr>'
const PORT = process.env.CDP_PORT || '9222';
const FILTER = process.env.CDP_FILTER || '';
const TIMEOUT = Number(process.env.CDP_TIMEOUT || 30000);

const expr = process.argv[2];
if (!expr) {
  console.error('uso: node tutorial/cdp.js "<espressione js>"');
  process.exit(2);
}

async function main() {
  const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
  const pages = list.filter((t) => t.type === 'page');
  const target = FILTER ? pages.find((t) => t.url.includes(FILTER)) : pages[0];
  if (!target) {
    console.error('nessuna pagina trovata. disponibili:');
    for (const p of pages) console.error(' -', p.url);
    process.exit(1);
  }

  const ws = new WebSocket(target.webSocketDebuggerUrl);
  let id = 0;
  const pending = new Map();

  const send = (method, params = {}) =>
    new Promise((resolve, reject) => {
      const msgId = ++id;
      pending.set(msgId, { resolve, reject });
      ws.send(JSON.stringify({ id: msgId, method, params }));
    });

  ws.addEventListener('message', (ev) => {
    const msg = JSON.parse(ev.data);
    if (msg.id && pending.has(msg.id)) {
      const { resolve, reject } = pending.get(msg.id);
      pending.delete(msg.id);
      if (msg.error) reject(new Error(JSON.stringify(msg.error)));
      else resolve(msg.result);
    }
  });

  await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve, { once: true });
    ws.addEventListener('error', () => reject(new Error('connessione CDP fallita')), { once: true });
  });

  const timer = setTimeout(() => {
    console.error('timeout CDP');
    process.exit(1);
  }, TIMEOUT);

  const res = await send('Runtime.evaluate', {
    expression: expr,
    awaitPromise: true,
    returnByValue: true,
    userGesture: true,
  });
  clearTimeout(timer);
  ws.close();

  if (res.exceptionDetails) {
    console.error('errore JS:', JSON.stringify(res.exceptionDetails, null, 2));
    process.exit(1);
  }
  const value = res.result.value;
  if (typeof value === 'string') console.log(value);
  else console.log(JSON.stringify(value, null, 2));
}

main().catch((e) => {
  console.error(String(e && e.message ? e.message : e));
  process.exit(1);
});
