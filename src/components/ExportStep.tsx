import { downloadText, exportZip, standaloneHtml } from '../workshop/exportGame';
import { createDefaultProject, projectJson } from '../workshop/project';
import type { Project } from '../types';

export function ExportStep({
  project,
  setProject,
  setNotice,
}: {
  project: Project;
  setProject: (project: Project) => void;
  setNotice: (notice: string) => void;
}) {
  return (
    <section className="step">
      <div className="panel">
        <h1>Take the game with you</h1>
        <p>
          The download is one HTML file with Emberleaf’s runtime inside it. Open it in Chrome, Firefox, or Edge on
          Linux, Windows, or Android. No emulator, and no store install. The zip adds a manifest and a short readme
          for Add to Home Screen, which needs the folder hosted over https. Opening the HTML file itself works directly.
        </p>
        <div className="row">
          <button
            type="button"
            className="btn primary"
            data-testid="download-html"
            onClick={() => downloadText(standaloneHtml(project), 'emberleaf.html', 'text/html')}
          >
            Download standalone HTML
          </button>
          <button
            type="button"
            className="btn"
            onClick={() => {
              void exportZip(project).then((blob) => {
                const url = URL.createObjectURL(blob);
                const link = document.createElement('a');
                link.href = url;
                link.download = 'emberleaf-game.zip';
                link.click();
                setTimeout(() => URL.revokeObjectURL(url), 1500);
              });
            }}
          >
            Download zip
          </button>
          <button type="button" className="btn" onClick={() => downloadText(projectJson(project), 'emberleaf-project.json', 'application/json')}>
            Download project JSON
          </button>
          <label className="btn">
            Load project JSON
            <input
              hidden
              type="file"
              accept="application/json,.json"
              onChange={async (event) => {
                const file = event.target.files?.[0];
                event.target.value = '';
                if (!file) return;
                try {
                  const parsed = JSON.parse(await file.text()) as Project;
                  if (!parsed || parsed.version !== 1 || !Array.isArray(parsed.levels)) {
                    setNotice('That file is not an Emberleaf project.');
                    return;
                  }
                  setProject(parsed);
                  setNotice('Project loaded. The raw ROM was never part of the save.');
                } catch {
                  setNotice('Could not read that JSON file.');
                }
              }}
            />
          </label>
          <button
            type="button"
            className="btn ghost"
            onClick={() => {
              setProject(createDefaultProject());
              setNotice('Sample workshop restored.');
            }}
          >
            Restore sample workshop
          </button>
        </div>
      </div>
    </section>
  );
}
