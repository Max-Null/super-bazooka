// DeepSeek API proxy — strips reasoning_effort from requests
// Proxies through the upstream proxy at HTTP_PROXY (127.0.0.1:7897)
const http = require('http');
const https = require('https');
const net = require('net');
const url = require('url');

const TARGET_HOST = 'api.deepseek.com';
const TARGET_PATH_PREFIX = '/anthropic';
const LISTEN_PORT = 7898;
const UPSTREAM_PROXY = process.env.HTTP_PROXY || 'http://127.0.0.1:7897';
const upstreamParsed = url.parse(UPSTREAM_PROXY);

function httpsAgentWithProxy() {
  return new https.Agent({
    keepAlive: true,
    createConnection: (opts, cb) => {
      const socket = net.connect(upstreamParsed.port || 7897, upstreamParsed.hostname);
      socket.on('connect', () => {
        // Send CONNECT for HTTPS tunnel with proxy-authorization if needed
        const connectReq = `CONNECT ${opts.hostname || opts.servername}:${opts.port || 443} HTTP/1.1\r\nHost: ${opts.hostname || opts.servername}\r\n\r\n`;
        socket.write(connectReq);
      });

      let data = '';
      socket.on('data', (chunk) => {
        data += chunk.toString();
        if (data.includes('\r\n\r\n')) {
          const status = parseInt(data.split(' ')[1]);
          if (status === 200) {
            // CONNECT succeeded, remaining data after headers becomes the TLS handshake
            const bodyStart = data.indexOf('\r\n\r\n') + 4;
            const remainder = data.slice(bodyStart);
            if (remainder.length > 0) {
              socket.unshift(Buffer.from(remainder));
            }
            cb(null, socket);
          } else {
            cb(new Error(`CONNECT failed: ${status}`));
          }
        }
      });
      socket.on('error', (err) => cb(err));
    }
  });
}

const agent = httpsAgentWithProxy();

const server = http.createServer((req, res) => {
  if (!req.url.startsWith(TARGET_PATH_PREFIX)) {
    res.writeHead(404);
    res.end('Not Found');
    return;
  }

  let body = '';
  req.on('data', chunk => { body += chunk; });
  req.on('end', () => {
    if (body) {
      try {
        const json = JSON.parse(body);
        if ('reasoning_effort' in json) {
          console.log(`[proxy] stripping reasoning_effort: ${json.reasoning_effort}`);
          delete json.reasoning_effort;
        }
        body = JSON.stringify(json);
      } catch (e) {}
    }

    // Copy relevant headers, remove host/proxy-specific ones
    const fwdHeaders = {};
    for (const [k, v] of Object.entries(req.headers)) {
      if (!['host', 'connection', 'proxy-connection', 'transfer-encoding'].includes(k.toLowerCase())) {
        fwdHeaders[k] = v;
      }
    }
    fwdHeaders['Content-Length'] = Buffer.byteLength(body);
    fwdHeaders['Host'] = TARGET_HOST;

    const options = {
      hostname: TARGET_HOST,
      port: 443,
      path: req.url,
      method: req.method,
      headers: fwdHeaders,
      agent: agent,
    };

    const proxyReq = https.request(options, (proxyRes) => {
      res.writeHead(proxyRes.statusCode, proxyRes.headers);
      proxyRes.pipe(res);
    });

    proxyReq.on('error', (err) => {
      console.error(`[proxy] error: ${err.message}`);
      res.writeHead(502);
      res.end('Proxy Error');
    });

    proxyReq.write(body);
    proxyReq.end();
  });
});

server.listen(LISTEN_PORT, '127.0.0.1', () => {
  console.log(`[proxy] listening on http://127.0.0.1:${LISTEN_PORT} -> ${TARGET_HOST} via ${UPSTREAM_PROXY}`);
});
