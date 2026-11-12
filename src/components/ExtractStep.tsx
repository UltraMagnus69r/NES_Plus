import { useEffect, useMemo, useRef, useState } from 'react';
import { decodeBrr, decodeDpcm, decodeGbWave, scanBrr, scanDpcm, scanGbWaves, wavDataUrl, type AudioHit } from '../rom/audio';
import { scanCandidates, suggestKind } from '../rom/classify';
import { PALETTES, decodeTile, paletteSize, renderTiles, tileBytes } from '../rom/codecs';
import { disassemble, hexDump, type CpuView } from '../rom/disasm';
import { findStrings } from '../rom/detect';
import { pngDataUrl } from '../lib/png';
import { uid } from '../lib/id';
import { defaultBehavior, defaultSolid, VISUAL_KINDS, type AssetKind, type CodecId, type Project, type RomSession, type Step } from '../types';
import { blockedName } from '../workshop/project';

const COLS = 16;
const ROWS = 12;

function cpuFor(format: string): CpuView {
  if (format === 'gb' || format === 'gbc') return 'lr35902';
  if (format === 'nes') return '6502';
  return 'hex';
}

export function ExtractStep({
  rom,
  setProject,
  setNotice,
  setStep,
}: {
  rom: RomSession | null;
  setProject: (recipe: (project: Project) => Project) => void;
  setNotice: (notice: string) => void;
  setStep: (step: Step) => void;
}) {
  const [tab, setTab] = useState<'tiles' | 'audio' | 'code'>('tiles');
  const [codec, setCodec] = useState<CodecId>(rom?.info.codec ?? 'nes');
  const [palette, setPalette] = useState<string[]>([...(PALETTES[rom?.info.codec ?? 'nes'] ?? PALETTES.nes)]);
  const [offset, setOffset] = useState(rom?.info.tileOffset ?? 0);
  const [selected, setSelected] = useState<number[]>([]);
  const [anchor, setAnchor] = useState<number | null>(null);
  const [name, setName] = useState('Pip part');
  const [kind, setKind] = useState<AssetKind>('enemy');
  const [clearZero, setClearZero] = useState(true);
  const [audioName, setAudioName] = useState('Sample');
  const [audioRole, setAudioRole] = useState<'sfx' | 'music'>('sfx');
  const [codeAt, setCodeAt] = useState(0);
  const [cpu, setCpu] = useState<CpuView>(cpuFor(rom?.info.format ?? 'unknown'));
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    if (!rom) return;
    setCodec(rom.info.codec);
    setPalette([...(PALETTES[rom.info.codec] ?? PALETTES.nes)]);
    setOffset(rom.info.tileOffset);
    setSelected([]);
    setCpu(cpuFor(rom.info.format));
    setCodeAt(rom.info.regions.find((region) => region.label === 'PRG')?.offset ?? 0);
  }, [rom]);

  const stride = tileBytes(codec);
  const origin = rom ? rom.info.tileOffset - (rom.info.tileOffset % stride) : 0;
  const rowBytes = stride * COLS;
  const page = rom ? Math.max(0, Math.floor((offset - origin) / rowBytes)) : 0;
  const pageStart = origin + page * rowBytes;

  const candidates = useMemo(() => (rom ? scanCandidates(rom.bytes, origin, codec) : []), [rom, origin, codec]);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas || !rom) return;
    const indices = new Uint8Array(COLS * ROWS * 64);
    for (let i = 0; i < COLS * ROWS; i += 1) {
      const tile = decodeTile(rom.bytes, pageStart + i * stride, codec);
      if (tile) indices.set(tile, i * 64);
    }
    const scale = 3;
    const view = renderTiles(indices, COLS, ROWS, palette, scale, clearZero);
    canvas.width = view.width;
    canvas.height = view.height;
    const ctx = canvas.getContext('2d');
    if (!ctx) return;
    ctx.imageSmoothingEnabled = false;
    const image = new ImageData(new Uint8ClampedArray(view.rgba), view.width, view.height);
    ctx.putImageData(image, 0, 0);
    ctx.strokeStyle = '#e25b2a';
    ctx.lineWidth = 2;
    const base = Math.floor(pageStart / stride);
    for (const index of selected) {
      const local = index - base;
      if (local < 0 || local >= COLS * ROWS) continue;
      const col = local % COLS;
      const row = Math.floor(local / COLS);
      ctx.strokeRect(col * 8 * scale + 1, row * 8 * scale + 1, 8 * scale - 2, 8 * scale - 2);
    }
  }, [rom, pageStart, stride, codec, palette, clearZero, selected]);

  if (!rom) {
    return (
      <section className="step">
        <div className="panel">
          <h1>Nothing is open</h1>
          <p>Load a dump or the demo cartridge from Source. Bytes stay in this tab.</p>
          <button type="button" className="btn primary" onClick={() => setStep('source')}>Go to Source</button>
        </div>
      </section>
    );
  }

  function toggle(index: number, shift: boolean) {
    if (shift && anchor !== null) {
      const [lo, hi] = anchor < index ? [anchor, index] : [index, anchor];
      const range = [];
      for (let i = lo; i <= hi; i += 1) range.push(i);
      setSelected(range);
      return;
    }
    setAnchor(index);
    setSelected((current) => (current.includes(index) ? current.filter((item) => item !== index) : [...current, index]));
  }

  function assembly(): { url: string; w: number; h: number } | null {
    if (!selected.length || !rom) return null;
    const base = Math.floor(origin / stride);
    const places = selected.map((index) => {
      const rel = index - base;
      const row = Math.floor(rel / COLS);
      const col = rel - row * COLS;
      return { index, col, row };
    });
    const minC = Math.min(...places.map((p) => p.col));
    const minR = Math.min(...places.map((p) => p.row));
    const maxC = Math.max(...places.map((p) => p.col));
    const maxR = Math.max(...places.map((p) => p.row));
    const tilesX = maxC - minC + 1;
    const tilesY = maxR - minR + 1;
    if (tilesX > 32 || tilesY > 32) return null;
    const indices = new Uint8Array(tilesX * tilesY * 64);
    for (const place of places) {
      const tile = decodeTile(rom.bytes, place.index * stride, codec);
      if (!tile) continue;
      indices.set(tile, ((place.row - minR) * tilesX + (place.col - minC)) * 64);
    }
    const view = renderTiles(indices, tilesX, tilesY, palette, 1, clearZero);
    return { url: pngDataUrl(view.rgba, view.width, view.height), w: tilesX * 8, h: tilesY * 8 };
  }

  function addSelection() {
    const made = assembly();
    if (!made) {
      setNotice('Select one or more tiles first.');
      return;
    }
    const blocked = blockedName(name);
    if (blocked) {
      setNotice(blocked);
      return;
    }
    const part = {
      id: uid('part'),
      name: name || 'Part',
      kind,
      src: made.url,
      w: made.w,
      h: made.h,
      behavior: defaultBehavior(kind),
      solid: defaultSolid(kind),
    };
    setProject((project) => ({ ...project, assets: [...project.assets, part] }));
    setNotice(`“${part.name}” is in the library as ${kind}.`);
  }

  function downloadPng() {
    const made = assembly();
    if (!made) return;
    const link = document.createElement('a');
    link.href = made.url;
    link.download = `${name || 'tiles'}.png`;
    link.click();
  }

  function addAudio(hit: AudioHit) {
    const blocked = blockedName(audioName);
    if (blocked) {
      setNotice(blocked);
      return;
    }
    const src = wavDataUrl(hit.samples, hit.rate);
    setProject((project) => ({
      ...project,
      assets: [...project.assets, { id: uid('audio'), name: audioName || hit.label, kind: audioRole, src, w: 0, h: 0, behavior: 'none' as const }],
    }));
    setNotice(`Added ${audioRole} “${audioName || hit.label}”.`);
  }

  async function preview(hit: AudioHit) {
    const ctx = new AudioContext();
    const buffer = ctx.createBuffer(1, hit.samples.length, hit.rate);
    buffer.getChannelData(0).set(hit.samples);
    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.connect(ctx.destination);
    src.start();
  }

  const strings = findStrings(rom.bytes);
  const lines = cpu === 'hex' ? [] : disassemble(rom.bytes, cpu, codeAt, 40);
  const dump = hexDump(rom.bytes, codeAt, 96);
  const hits = codec === 'gb' ? scanGbWaves(rom.bytes) : codec === 'snes' ? scanBrr(rom.bytes) : scanDpcm(rom.bytes, 0, Math.min(rom.bytes.length, 512 * 1024));
  const made = assembly();

  return (
    <section className="step">
      <div className="panel">
        <div className="row">
          <h1 style={{ marginRight: 12 }}>{rom.info.title || rom.name}</h1>
          {(['tiles', 'audio', 'code'] as const).map((item) => (
            <button key={item} type="button" className={tab === item ? 'btn primary' : 'btn'} onClick={() => setTab(item)}>{item}</button>
          ))}
        </div>
        <div className="row">
          {rom.info.details.map((detail) => (
            <span key={detail.label} className="tag">{detail.label}: {detail.value}</span>
          ))}
        </div>
        {rom.info.warnings.map((warning) => <p key={warning} className="warn">{warning}</p>)}
      </div>
      {tab === 'tiles' && (
        <div className="grid-2">
          <div className="panel">
            <div className="row">
              <label className="field">Codec
                <select value={codec} onChange={(event) => {
                  const next = event.target.value as CodecId;
                  setCodec(next);
                  setPalette([...(PALETTES[next] ?? PALETTES.nes)].slice(0, paletteSize(next)));
                }}>
                  <option value="nes">NES 2bpp</option>
                  <option value="gb">GB 2bpp</option>
                  <option value="snes">SNES 4bpp</option>
                  <option value="gba">GBA 4bpp</option>
                  <option value="1bpp">1bpp</option>
                </select>
              </label>
              {rom.info.regions.map((region) => (
                <button key={region.label} type="button" className="btn" onClick={() => setOffset(region.offset)}>{region.label}</button>
              ))}
              <button type="button" className="btn" onClick={() => setOffset(Math.max(origin, pageStart - rowBytes))}>Prev</button>
              <button type="button" className="btn" onClick={() => setOffset(pageStart + rowBytes)}>Next</button>
              <span className="tag">offset ${pageStart.toString(16)}</span>
            </div>
            <canvas
              ref={canvasRef}
              className="tiles"
              onClick={(event) => {
                const canvas = canvasRef.current;
                if (!canvas) return;
                const rect = canvas.getBoundingClientRect();
                const scale = 3;
                const x = ((event.clientX - rect.left) * canvas.width) / rect.width;
                const y = ((event.clientY - rect.top) * canvas.height) / rect.height;
                const col = Math.floor(x / (8 * scale));
                const row = Math.floor(y / (8 * scale));
                if (col < 0 || row < 0 || col >= COLS || row >= ROWS) return;
                const index = Math.floor(pageStart / stride) + row * COLS + col;
                const tile = decodeTile(rom.bytes, index * stride, codec);
                if (tile) {
                  const guess = suggestKind(tile);
                  if (guess === 'sprite') setKind('enemy');
                  else if (guess === 'scenery') setKind('terrain');
                  else if (guess === 'text') setKind('foreground');
                }
                toggle(index, event.shiftKey);
              }}
            />
            <div className="candidates">
              {candidates.slice(0, 48).map((candidate) => {
                const tile = decodeTile(rom.bytes, candidate.offset, codec);
                if (!tile) return null;
                const view = renderTiles(tile, 1, 1, palette, 4, clearZero);
                const url = pngDataUrl(view.rgba, view.width, view.height);
                return (
                  <button key={candidate.offset} type="button" className={selected.includes(candidate.index) ? 'candidate on' : 'candidate'} onClick={() => {
                    const rel = candidate.index - Math.floor(origin / stride);
                    const row = Math.max(0, Math.floor(rel / COLS));
                    const pageRow = Math.floor(row / ROWS) * ROWS;
                    setOffset(origin + pageRow * rowBytes);
                    setSelected([candidate.index]);
                    setAnchor(candidate.index);
                    if (candidate.kind === 'sprite') setKind('enemy');
                    if (candidate.kind === 'scenery') setKind('terrain');
                  }}>
                    <img src={url} alt="" />
                    <div className="tag">{candidate.kind} ×{candidate.count}</div>
                  </button>
                );
              })}
            </div>
          </div>
          <aside className="panel">
            <h2>Make a part</h2>
            <p>Tiles are not labeled in the ROM. A guess is only a guess. You name the part and choose what it means in the game.</p>
            <div className="swatches">
              {palette.slice(0, paletteSize(codec)).map((color, index) => (
                <input key={index} type="color" value={color} aria-label={`Palette ${index}`} onChange={(event) => setPalette(palette.map((item, i) => (i === index ? event.target.value : item)))} />
              ))}
            </div>
            <label className="row"><input type="checkbox" checked={clearZero} onChange={(event) => setClearZero(event.target.checked)} /> Index 0 is transparent</label>
            {made ? <img src={made.url} alt="" width={made.w * 4} height={made.h * 4} style={{ imageRendering: 'pixelated' }} /> : <p>Select tiles on the sheet. Shift-click selects a range.</p>}
            <label className="field">Name
              <input value={name} onChange={(event) => setName(event.target.value)} />
            </label>
            <label className="field">Kind
              <select value={kind} onChange={(event) => setKind(event.target.value as AssetKind)}>
                {VISUAL_KINDS.map((option) => <option key={option} value={option}>{option}</option>)}
              </select>
            </label>
            <div className="row">
              <button type="button" className="btn primary" data-testid="add-to-library" onClick={addSelection}>Add to library</button>
              <button type="button" className="btn" onClick={downloadPng}>Download PNG</button>
            </div>
          </aside>
        </div>
      )}
      {tab === 'audio' && (
        <div className="panel">
          <h2>Uncompressed audio</h2>
          <p>
            NES DPCM is LSB-first, starting at level 64, stepping by 2. Game Boy wave RAM is 16 bytes of nibbles. Super NES BRR uses the integer S-DSP filters, high nibble first. These scans only see bytes that already decode. Directories and compression are left to other tools. Preview before you keep one.
          </p>
          <div className="row">
            <label className="field">Name<input value={audioName} onChange={(event) => setAudioName(event.target.value)} /></label>
            <label className="field">Keep as
              <select value={audioRole} onChange={(event) => setAudioRole(event.target.value as 'sfx' | 'music')}>
                <option value="sfx">sfx</option>
                <option value="music">music</option>
              </select>
            </label>
            <button type="button" className="btn" onClick={() => preview({ offset: codeAt, encoding: 'dpcm', samples: codec === 'gb' ? decodeGbWave(rom.bytes.subarray(codeAt, codeAt + 16)) : codec === 'snes' ? decodeBrr(rom.bytes.subarray(codeAt, codeAt + 9 * 16)) : decodeDpcm(rom.bytes.subarray(codeAt, codeAt + 256)), rate: 8000, score: 0, label: 'slice' })}>
              Preview slice at code offset
            </button>
          </div>
          <div className="cards">
            {hits.map((hit) => (
              <article key={`${hit.encoding}-${hit.offset}`} className="card">
                <strong>{hit.label}</strong>
                <span className="tag">score {hit.score.toFixed(3)}</span>
                <div className="row">
                  <button type="button" className="btn" onClick={() => void preview(hit)}>Preview</button>
                  <button type="button" className="btn primary" onClick={() => addAudio(hit)}>Add</button>
                </div>
              </article>
            ))}
            {!hits.length ? <p>No confident candidates in the scanned window. You can still preview a slice.</p> : null}
          </div>
        </div>
      )}
      {tab === 'code' && (
        <div className="panel">
          <p className="warn">
            Scripts in these games are custom encodings, and physics is not a standard file format. Emberleaf lists ASCII strings and shows a machine-code window — 6502 for NES, LR35902 for Game Boy. It does not recover a script, and it does not decompile physics. The platforming in Play is Emberleaf’s own runtime. Super NES code is 65816 and Game Boy Advance code is ARM; those windows stay a hex dump so we do not pretend otherwise.
          </p>
          <div className="row">
            <label className="field">CPU
              <select value={cpu} onChange={(event) => setCpu(event.target.value as CpuView)}>
                <option value="6502">6502</option>
                <option value="lr35902">LR35902</option>
                <option value="hex">Hex only</option>
              </select>
            </label>
            <label className="field">File offset
              <input type="number" value={codeAt} min={0} max={Math.max(0, rom.bytes.length - 1)} onChange={(event) => setCodeAt(Number(event.target.value) || 0)} />
            </label>
          </div>
          <div className="grid-2">
            <div className="codebox mono">
              {lines.map((line) => (
                <div key={`${line.address}-${line.bytes}`}>{line.address.toString(16).padStart(6, '0')}  {line.bytes.padEnd(12, ' ')} {line.text}</div>
              ))}
              {cpu === 'hex' ? dump.map((row) => (
                <div key={row.offset}>{row.offset.toString(16).padStart(6, '0')}  {row.hex.padEnd(48, ' ')} {row.ascii}</div>
              )) : null}
            </div>
            <div className="codebox">
              {strings.length ? strings.map((item) => (
                <button key={`${item.offset}-${item.text}`} type="button" className="btn ghost" onClick={() => setCodeAt(item.offset)}>
                  ${item.offset.toString(16)} {item.text}
                </button>
              )) : <p>No ASCII strings of length 6 or more in the first 2 MB.</p>}
            </div>
          </div>
        </div>
      )}
    </section>
  );
}

