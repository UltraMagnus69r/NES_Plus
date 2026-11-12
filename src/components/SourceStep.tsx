import { useState } from 'react';
import { detectRom } from '../rom/detect';
import { defaultBehavior, defaultSolid, type AssetKind, type Project, type RomSession, type Step } from '../types';
import { uid } from '../lib/id';
import { blockedName, buildDemoCartridge } from '../workshop/project';

const MAX_BYTES = 64 * 1024 * 1024;

type Props = {
  project: Project;
  setProject: (project: Project) => void;
  setRom: (rom: RomSession | null) => void;
  setStep: (step: Step) => void;
  setNotice: (notice: string) => void;
};

async function bytesOf(file: File): Promise<Uint8Array> {
  if (file.size > MAX_BYTES) throw new Error('Emberleaf opens files up to 64 MB. This one is larger.');
  return new Uint8Array(await file.arrayBuffer());
}

function loadImage(src: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('Could not read that image.'));
    image.src = src;
  });
}

export function SourceStep({ project, setProject, setRom, setStep, setNotice }: Props) {
  const [hot, setHot] = useState(false);
  const [slice, setSlice] = useState(true);

  async function take(file: File) {
    setNotice('');
    const name = file.name;
    const lower = name.toLowerCase();
    const isImage = file.type.startsWith('image/') || /\.(png|gif|webp)$/.test(lower);
    const isAudio = file.type.startsWith('audio/') || /\.(wav|mp3|ogg|flac)$/.test(lower);
    if (isImage) {
      const raw = await bytesOf(file);
      let binary = '';
      for (let i = 0; i < raw.length; i += 0x8000) binary += String.fromCharCode(...raw.subarray(i, i + 0x8000));
      const src = `data:${file.type || 'image/png'};base64,${btoa(binary)}`;
      await addImage(src, name.replace(/\.[^.]+$/, '') || 'Sprite');
      return;
    }
    if (isAudio) {
      if (file.size > 4 * 1024 * 1024) {
        setNotice('Audio imports are kept under 4 MB so the workshop can autosave.');
        return;
      }
      const raw = await bytesOf(file);
      let binary = '';
      for (let i = 0; i < raw.length; i += 0x8000) binary += String.fromCharCode(...raw.subarray(i, i + 0x8000));
      const src = `data:${file.type || 'audio/wav'};base64,${btoa(binary)}`;
      const partName = name.replace(/\.[^.]+$/, '') || 'Audio';
      const blocked = blockedName(partName);
      if (blocked) {
        setNotice(blocked);
        return;
      }
      setProject({
        ...project,
        assets: [...project.assets, { id: uid('audio'), name: partName, kind: 'music', src, w: 0, h: 0, behavior: 'none' }],
      });
      setNotice(`Added “${partName}” to the library as music. Change it to a sound effect there if you like.`);
      setStep('library');
      return;
    }
    const bytes = await bytesOf(file);
    const info = detectRom(bytes, name);
    setRom({ name, bytes, info });
    setNotice(info.warnings[0] ?? `${info.details[0]?.value ?? name} is open in this browser only.`);
    setStep('extract');
  }

  async function addImage(src: string, partName: string) {
    const blocked = blockedName(partName);
    if (blocked) {
      setNotice(blocked);
      return;
    }
    const image = await loadImage(src);
    const kind: AssetKind = 'terrain';
    const additions = [];
    if (slice && image.width >= 32 && image.height >= 16 && image.width % 16 === 0 && image.height % 16 === 0) {
      const canvas = document.createElement('canvas');
      canvas.width = 16;
      canvas.height = 16;
      const ctx = canvas.getContext('2d');
      if (!ctx) return;
      ctx.imageSmoothingEnabled = false;
      let n = 0;
      for (let y = 0; y < image.height; y += 16) {
        for (let x = 0; x < image.width; x += 16) {
          ctx.clearRect(0, 0, 16, 16);
          ctx.drawImage(image, x, y, 16, 16, 0, 0, 16, 16);
          const data = ctx.getImageData(0, 0, 16, 16).data;
          if ([...data].every((v, i) => i % 4 !== 3 || v === 0)) continue;
          n += 1;
          additions.push({
            id: uid('part'),
            name: `${partName} ${n}`,
            kind,
            src: canvas.toDataURL('image/png'),
            w: 16,
            h: 16,
            behavior: defaultBehavior(kind),
            solid: defaultSolid(kind),
          });
        }
      }
    }
    if (!additions.length) {
      additions.push({
        id: uid('part'),
        name: partName,
        kind: 'foreground' as const,
        src,
        w: image.width,
        h: image.height,
        behavior: defaultBehavior('foreground'),
        solid: false,
      });
    }
    setProject({ ...project, assets: [...project.assets, ...additions] });
    setNotice(`Added ${additions.length} image part${additions.length === 1 ? '' : 's'}. Set the kind in the library.`);
    setStep('library');
  }

  return (
    <section className="step">
      <div className="panel">
        <h1>Bring a cartridge, or start from the grove.</h1>
        <p className="rights">
          Bring dumps you may use. Emberleaf includes no Nintendo sprites, music, ROMs, or code. Files stay in this
          browser session and are never uploaded. Classic unencrypted NES, Game Boy, Game Boy Color, Super NES, and
          Game Boy Advance dumps can be paged here. Switch, 3DS, Wii U, and other encrypted systems are not supported.
          Files over 64 MB are refused.
        </p>
        <div
          className={hot ? 'drop hot' : 'drop'}
          onDragOver={(event) => {
            event.preventDefault();
            setHot(true);
          }}
          onDragLeave={() => setHot(false)}
          onDrop={(event) => {
            event.preventDefault();
            setHot(false);
            const file = event.dataTransfer.files[0];
            if (file) void take(file).catch((error: Error) => setNotice(error.message));
          }}
        >
          <div>
            <h2>Drop a ROM, image, or audio file</h2>
            <p>Or use the buttons. The sample courses are already in the library.</p>
          </div>
        </div>
        <div className="row" style={{ marginTop: 12 }}>
          <button
            type="button"
            className="btn primary"
            data-testid="load-demo"
            onClick={() => {
              const bytes = buildDemoCartridge();
              const info = detectRom(bytes, 'emberleaf-demo.nes');
              setRom({ name: 'emberleaf-demo.nes', bytes, info });
              setNotice('Demo cartridge is original Emberleaf CHR. Nothing from Nintendo is in it.');
              setStep('extract');
            }}
          >
            Load demo cartridge
          </button>
          <label className="btn">
            Import PNG sprites
            <input
              type="file"
              accept="image/png,image/gif,image/webp"
              hidden
              onChange={(event) => {
                const file = event.target.files?.[0];
                if (file) void take(file).catch((error: Error) => setNotice(error.message));
                event.target.value = '';
              }}
            />
          </label>
          <label className="btn">
            Import audio
            <input
              type="file"
              accept="audio/*"
              hidden
              onChange={(event) => {
                const file = event.target.files?.[0];
                if (file) void take(file).catch((error: Error) => setNotice(error.message));
                event.target.value = '';
              }}
            />
          </label>
          <label className="field">
            <span>Slice large PNGs into 16×16</span>
            <input type="checkbox" checked={slice} onChange={(event) => setSlice(event.target.checked)} />
          </label>
          <button type="button" className="btn ghost" onClick={() => setStep('levels')}>
            Open the sample courses
          </button>
        </div>
      </div>
    </section>
  );
}
