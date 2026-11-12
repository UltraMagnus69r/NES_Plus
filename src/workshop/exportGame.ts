import JSZip from 'jszip';
import playerSource from '../runtime/player.js?raw';
import type { Project } from '../types';

function safeJson(value: unknown): string {
  return JSON.stringify(value).replace(/</g, '\\u003c').replace(/\u2028/g, '\\u2028').replace(/\u2029/g, '\\u2029');
}

export function standaloneHtml(project: Project): string {
  const title = project.name || 'Emberleaf';
  return `<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<meta name="theme-color" content="#12110f">
<title>${title.replace(/[&<>]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;' })[c] || c)}</title>
<link rel="manifest" href="manifest.webmanifest">
<style>
  html, body { margin: 0; height: 100%; background: #12110f; color: #efe6d6; }
  body { font-family: "Instrument Sans", system-ui, sans-serif; }
  canvas { width: 100%; height: 100%; display: block; touch-action: none; background: #12110f; }
</style>
</head>
<body>
<canvas id="screen"></canvas>
<script type="module">
${playerSource}
const project = ${safeJson(project)};
const canvas = document.querySelector('#screen');
runGame(canvas, project, {});
</script>
</body>
</html>
`;
}

export function manifestFor(project: Project): string {
  return JSON.stringify({
    name: project.name || 'Emberleaf',
    short_name: 'Emberleaf',
    display: 'standalone',
    background_color: '#12110f',
    theme_color: '#e25b2a',
    start_url: './index.html',
  }, null, 2);
}

export function exportReadme(project: Project): string {
  return `${project.name || 'Emberleaf'}

Open index.html in Chrome, Firefox, or Edge. That works directly on Linux, Windows, and Android. No emulator and no install.

Add to Home Screen only works after this folder is hosted over https. Opening the HTML file itself does not need a server.

The game is Emberleaf's own runtime. It does not contain Nintendo code.
`;
}

export async function exportZip(project: Project): Promise<Blob> {
  const zip = new JSZip();
  zip.file('index.html', standaloneHtml(project));
  zip.file('manifest.webmanifest', manifestFor(project));
  zip.file('README.txt', exportReadme(project));
  return zip.generateAsync({ type: 'blob' });
}

export function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1500);
}

export function downloadText(text: string, filename: string, type = 'text/plain') {
  downloadBlob(new Blob([text], { type }), filename);
}
