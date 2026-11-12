import { useCallback, useEffect, useState } from 'react';
import { ExportStep } from './components/ExportStep';
import { ExtractStep } from './components/ExtractStep';
import { GuideStep } from './components/GuideStep';
import { LevelStep } from './components/LevelStep';
import { LibraryStep } from './components/LibraryStep';
import { OverworldStep } from './components/OverworldStep';
import { PlayStep } from './components/PlayStep';
import { SourceStep } from './components/SourceStep';
import type { Project, RomSession, Step } from './types';
import { loadProject, saveProject } from './workshop/project';

const STEPS: { id: Step; label: string }[] = [
  { id: 'source', label: 'Source' },
  { id: 'extract', label: 'Extract' },
  { id: 'library', label: 'Library' },
  { id: 'levels', label: 'Levels' },
  { id: 'overworld', label: 'Overworld' },
  { id: 'play', label: 'Play' },
  { id: 'export', label: 'Export' },
  { id: 'guide', label: 'Guide' },
];

export function App() {
  const [step, setStep] = useState<Step>('levels');
  const [project, setProject] = useState<Project>(() => loadProject());
  const [rom, setRom] = useState<RomSession | null>(null);
  const [courseId, setCourseId] = useState<string | null>(null);
  const [saved, setSaved] = useState('Saved locally');
  const [notice, setNotice] = useState('');

  useEffect(() => {
    setSaved('Saving…');
    const timer = window.setTimeout(() => {
      setSaved(saveProject(project) === 'saved' ? 'Saved locally' : 'Could not autosave');
    }, 250);
    return () => window.clearTimeout(timer);
  }, [project]);

  const exitPlay = useCallback(() => setStep(courseId ? 'levels' : 'overworld'), [courseId]);

  return (
    <div className="app">
      <header className="topbar">
        <div className="mark">
          <span className="seed" aria-hidden="true" />
          <div>
            <strong>Emberleaf</strong>
            <em>workshop</em>
          </div>
        </div>
        <nav className="nav">
          {STEPS.map((item) => (
            <button
              key={item.id}
              type="button"
              className={step === item.id ? 'active' : ''}
              data-testid={`nav-${item.id}`}
              onClick={() => {
                if (item.id === 'play') setCourseId(null);
                setStep(item.id);
              }}
            >
              {item.label}
            </button>
          ))}
        </nav>
        <div className="save">{saved}</div>
      </header>
      {notice ? <p className="notice">{notice}</p> : null}
      {step === 'source' && <SourceStep project={project} setProject={setProject} setRom={setRom} setStep={setStep} setNotice={setNotice} />}
      {step === 'extract' && <ExtractStep rom={rom} setProject={setProject} setNotice={setNotice} setStep={setStep} />}
      {step === 'library' && <LibraryStep project={project} setProject={setProject} setNotice={setNotice} />}
      {step === 'levels' && (
        <LevelStep
          project={project}
          setProject={setProject}
          onPlay={(id) => {
            setCourseId(id);
            setStep('play');
          }}
        />
      )}
      {step === 'overworld' && <OverworldStep project={project} setProject={setProject} />}
      {step === 'play' && <PlayStep project={project} courseId={courseId} onExit={exitPlay} />}
      {step === 'export' && <ExportStep project={project} setProject={setProject} setNotice={setNotice} />}
      {step === 'guide' && <GuideStep />}
    </div>
  );
}
