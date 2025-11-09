const express = require('express');
const app = express();
const port = process.env.PORT || 3000;

app.get('/api/health', (req, res) => res.json({ status: 'ok' }));

app.get('/', (req, res) => {
  res.send(`
    <html>
      <head><title>Span Dashboard</title></head>
      <body style="font-family: sans-serif; padding: 2rem;">
        <h1>Span Dashboard</h1>
        <p>Welcome to Span. Control Plane URL: ${process.env.CONTROL_PLANE_URL || 'http://localhost:8080'}</p>
      </body>
    </html>
  `);
});

app.listen(port, () => console.log(`Dashboard listening on ${port}`));
