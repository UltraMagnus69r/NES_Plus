import { useRef, useState } from 'react';
import type { OverNode, Project } from '../types';
import { uid } from '../lib/id';

export function OverworldStep({ project, setProject }: { project: Project; setProject: (project: Project) => void }) {
  const [selected, setSelected] = useState(project.startNodeId);
  const [linking, setLinking] = useState(false);
  const drag = useRef<{ id: string; dx: number; dy: number } | null>(null);
  const node = project.nodes.find((item) => item.id === selected) ?? project.nodes[0];

  function patch(id: string, recipe: (item: OverNode) => OverNode) {
    setProject({ ...project, nodes: project.nodes.map((item) => (item.id === id ? recipe(item) : item)) });
  }

  function toggleLink(a: string, b: string) {
    if (a === b) return;
    setProject({
      ...project,
      nodes: project.nodes.map((item) => {
        if (item.id !== a && item.id !== b) return item;
        const other = item.id === a ? b : a;
        const has = item.links.includes(other);
        return { ...item, links: has ? item.links.filter((id) => id !== other) : [...item.links, other] };
      }),
    });
  }

  return (
    <section className="step">
      <div className="grid-2">
        <div className="panel">
          <h1>Overworld</h1>
          <p>Drag a node. Link two nodes so clearing one opens the other. The start node is playable first. The game is won when every linked course is clear.</p>
          <div
            className="map-stage"
            data-testid="overworld-map"
            onPointerMove={(event) => {
              if (!drag.current) return;
              const rect = event.currentTarget.getBoundingClientRect();
              const x = Math.round((event.clientX - rect.left - drag.current.dx) / 2);
              const y = Math.round((event.clientY - rect.top - drag.current.dy) / 2);
              patch(drag.current.id, (item) => ({ ...item, x: Math.max(16, Math.min(300, x)), y: Math.max(20, Math.min(160, y)) }));
            }}
            onPointerUp={() => {
              drag.current = null;
            }}
          >
            <svg>
              {project.nodes.flatMap((item) =>
                item.links.filter((id) => id > item.id).map((id) => {
                  const other = project.nodes.find((candidate) => candidate.id === id);
                  if (!other) return null;
                  return <line key={`${item.id}-${id}`} x1={item.x * 2} y1={item.y * 2} x2={other.x * 2} y2={other.y * 2} stroke="#7d9a62" strokeWidth="4" />;
                }),
              )}
            </svg>
            {project.nodes.map((item) => (
              <button
                key={item.id}
                type="button"
                className={`node${item.id === project.startNodeId ? ' start' : ''}${item.id === selected ? ' sel' : ''}`}
                style={{ left: item.x * 2, top: item.y * 2 }}
                onPointerDown={(event) => {
                  const rect = (event.currentTarget.parentElement as HTMLElement).getBoundingClientRect();
                  drag.current = { id: item.id, dx: event.clientX - rect.left - item.x * 2, dy: event.clientY - rect.top - item.y * 2 };
                  event.currentTarget.setPointerCapture(event.pointerId);
                  if (linking && selected && selected !== item.id) toggleLink(selected, item.id);
                  setSelected(item.id);
                  setLinking(false);
                }}
              >
                <span>{item.name}</span>
              </button>
            ))}
          </div>
        </div>
        <aside className="panel">
          {node ? (
            <>
              <h2>{node.name}</h2>
              <label className="field">
                Label
                <input value={node.name} onChange={(event) => patch(node.id, (item) => ({ ...item, name: event.target.value }))} />
              </label>
              <label className="field">
                Course
                <select value={node.levelId} onChange={(event) => patch(node.id, (item) => ({ ...item, levelId: event.target.value }))}>
                  {project.levels.map((level) => (
                    <option key={level.id} value={level.id}>{level.name}</option>
                  ))}
                </select>
              </label>
              <div className="row" style={{ marginTop: 10 }}>
                <button type="button" className="btn" onClick={() => setProject({ ...project, startNodeId: node.id })}>
                  {project.startNodeId === node.id ? 'Start node' : 'Make start'}
                </button>
                <button type="button" className={linking ? 'btn primary' : 'btn'} onClick={() => setLinking(true)}>
                  Link to another node
                </button>
                <button
                  type="button"
                  className="btn"
                  onClick={() => {
                    const id = uid('node');
                    setProject({
                      ...project,
                      nodes: [...project.nodes, { id, name: 'New path', levelId: project.levels[0]?.id ?? '', x: 140, y: 90, links: [] }],
                    });
                    setSelected(id);
                  }}
                >
                  Add node
                </button>
                {project.nodes.length > 1 ? (
                  <button
                    type="button"
                    className="btn ghost"
                    onClick={() => {
                      const nodes = project.nodes
                        .filter((item) => item.id !== node.id)
                        .map((item) => ({ ...item, links: item.links.filter((id) => id !== node.id) }));
                      setProject({ ...project, nodes, startNodeId: project.startNodeId === node.id ? nodes[0].id : project.startNodeId });
                      setSelected(nodes[0].id);
                    }}
                  >
                    Remove node
                  </button>
                ) : null}
              </div>
            </>
          ) : null}
        </aside>
      </div>
    </section>
  );
}
