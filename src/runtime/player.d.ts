export const TILE: number;

export function makeWorld(spec?: Record<string, unknown>): SimWorld;
export function stepWorld(world: SimWorld, input?: SimInput): SimWorld;
export function buildWorld(level: RuntimeLevel, assets: RuntimeAsset[]): SimWorld;
export function isSolidTile(world: SimWorld, tx: number, ty: number): boolean;
export function runGame(canvas: HTMLCanvasElement, project: RuntimeProject, options?: RunOptions): () => void;
export function previewSound(preset: string): void;

export type SimInput = {
  left?: boolean;
  right?: boolean;
  jump?: boolean;
  jumpPressed?: boolean;
  jumpReleased?: boolean;
};

export type SimBody = {
  x: number;
  y: number;
  vx: number;
  vy: number;
  w: number;
  h: number;
  onGround: boolean;
  facing: number;
  coyote: number;
  jumpBuffer: number;
  alive: boolean;
  hurtCooldown: number;
};

export type SimActor = {
  id: string;
  assetId: string;
  x: number;
  y: number;
  w: number;
  h: number;
  dir: number;
  speed: number;
  behavior: string;
  alive: boolean;
};

export type SimWorld = {
  tileSize: number;
  width: number;
  height: number;
  solids: Uint8Array;
  hazards: Uint8Array;
  actors: SimActor[];
  initialActors: SimActor[];
  player: SimBody;
  spawn: { x: number; y: number };
  completed: boolean;
  deadTimer: number;
  events: string[];
  gravity: number;
  maxFall: number;
  runAccel: number;
  maxRun: number;
  friction: number;
  airFriction: number;
  jumpVelocity: number;
  jumpCut: number;
  coyoteFrames: number;
  jumpBufferFrames: number;
  stompBounce: number;
  stompSlop: number;
};

export type RuntimeAsset = {
  id: string;
  kind: string;
  src?: string;
  preset?: string;
  solid?: boolean;
  behavior?: string;
  w?: number;
  h?: number;
};

export type RuntimeLevel = {
  id: string;
  name: string;
  w: number;
  h: number;
  sky?: string;
  musicId?: string;
  player: { x: number; y: number };
  cells: { x: number; y: number; assetId: string; layer: string }[];
};

export type RuntimeProject = {
  name?: string;
  playerId?: string;
  assets: RuntimeAsset[];
  levels: RuntimeLevel[];
  nodes: { id: string; name: string; levelId: string; x: number; y: number; links: string[] }[];
  startNodeId: string;
};

export type RunOptions = {
  courseId?: string | null;
  onExit?: () => void;
  onClear?: (levelId: string) => void;
  touch?: boolean;
};
