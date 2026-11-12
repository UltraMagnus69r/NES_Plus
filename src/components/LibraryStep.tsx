import { previewSound } from '../runtime/player.js';
import { ASSET_KINDS, type Asset, type Behavior, type Project } from '../types';
import { blockedName } from '../workshop/project';

const BEHAVIORS: Behavior[] = ['patrol', 'collect', 'hurt', 'goal', 'player', 'decoration', 'none'];
const CUES = ['', 'jump', 'stomp', 'collect', 'hurt', 'goal', 'theme'];

export function LibraryStep({
  project,
  setProject,
  setNotice,
}: {
  project: Project;
  setProject: (project: Project) => void;
  setNotice: (notice: string) => void;
}) {
  function patch(id: string, recipe: (asset: Asset) => Asset) {
    setProject({ ...project, assets: project.assets.map((asset) => (asset.id === id ? recipe(asset) : asset)) });
  }

  return (
    <section className="step">
      <div className="panel">
        <h1>Library</h1>
        <p>Parts are grouped by the kind you gave them. Terrain can be solid. Actors use a behavior when a course runs.</p>
      </div>
      <div className="folders">
        {ASSET_KINDS.map((kind) => {
          const items = project.assets.filter((asset) => asset.kind === kind);
          if (!items.length) return null;
          return (
            <section key={kind} className="panel folder">
              <h3>{kind}</h3>
              <div className="cards">
                {items.map((asset) => (
                  <article key={asset.id} className="card">
                    {asset.src && asset.kind !== 'sfx' && asset.kind !== 'music' ? <img src={asset.src} alt="" width={48} height={48} /> : <div className="tag">{asset.preset || 'audio'}</div>}
                    <label className="field">
                      Name
                      <input
                        type="text"
                        value={asset.name}
                        onChange={(event) => {
                          const blocked = blockedName(event.target.value);
                          if (blocked) {
                            setNotice(blocked);
                            return;
                          }
                          patch(asset.id, (item) => ({ ...item, name: event.target.value }));
                        }}
                      />
                    </label>
                    <label className="field">
                      Kind
                      <select
                        value={asset.kind}
                        onChange={(event) => patch(asset.id, (item) => ({ ...item, kind: event.target.value as Asset['kind'] }))}
                      >
                        {ASSET_KINDS.map((option) => (
                          <option key={option} value={option}>{option}</option>
                        ))}
                      </select>
                    </label>
                    {asset.kind !== 'sfx' && asset.kind !== 'music' ? (
                      <>
                        <label className="field">
                          Behavior
                          <select
                            value={asset.behavior || 'decoration'}
                            onChange={(event) => patch(asset.id, (item) => ({ ...item, behavior: event.target.value as Behavior }))}
                          >
                            {BEHAVIORS.map((option) => (
                              <option key={option} value={option}>{option}</option>
                            ))}
                          </select>
                        </label>
                        <label className="row">
                          <input
                            type="checkbox"
                            checked={Boolean(asset.solid)}
                            onChange={(event) => patch(asset.id, (item) => ({ ...item, solid: event.target.checked }))}
                          />
                          Solid
                        </label>
                        {asset.kind === 'player' ? (
                          <button type="button" className="btn" onClick={() => setProject({ ...project, playerId: asset.id })}>
                            {project.playerId === asset.id ? 'This is the player' : 'Use as the player'}
                          </button>
                        ) : null}
                      </>
                    ) : (
                      <>
                        <label className="field">
                          Game cue
                          <select
                            value={asset.preset || ''}
                            onChange={(event) => patch(asset.id, (item) => ({ ...item, preset: event.target.value || undefined }))}
                          >
                            {CUES.map((cue) => (
                              <option key={cue || 'none'} value={cue}>{cue || 'none'}</option>
                            ))}
                          </select>
                        </label>
                        <button
                          type="button"
                          className="btn"
                          onClick={() => {
                            if (asset.src) {
                              const audio = new Audio(asset.src);
                              void audio.play();
                            } else if (asset.preset) previewSound(asset.preset);
                          }}
                        >
                          Preview
                        </button>
                      </>
                    )}
                    <button
                      type="button"
                      className="btn ghost"
                      onClick={() => setProject({ ...project, assets: project.assets.filter((item) => item.id !== asset.id) })}
                    >
                      Remove
                    </button>
                  </article>
                ))}
              </div>
            </section>
          );
        })}
      </div>
    </section>
  );
}
