// The send animation: the prompt's letters shrink into a pixel packet that arcs to the
// corner avatar. Pure decoration with the Web Animations API (compositor driven); the
// prompt was already written to the pty before this starts.

export interface PacketFlight {
  /** Element the animation is drawn in (position: relative). */
  host: HTMLElement;
  text: string;
  /** Where the text was (client coordinates) and how it looked. */
  textBox: DOMRect;
  textStyle: Partial<CSSStyleDeclaration>;
  /** Collapse point (client coordinates), e.g. the caret. */
  origin: { x: number; y: number };
  /** The corner avatar (client coordinates); null skips the flight. */
  target: DOMRect | null;
  /** Total duration in ms (~600). */
  duration: number;
  onArrive: () => void;
}

const ARC_STEPS = 12;

export function playPacket(flight: PacketFlight): void {
  const base = flight.host.getBoundingClientRect();
  const half = flight.duration / 2;
  const ghost = document.createElement('div');
  ghost.className = 'af-packet-text';
  ghost.textContent = flight.text;
  Object.assign(ghost.style, flight.textStyle, {
    left: `${flight.textBox.left - base.left}px`,
    top: `${flight.textBox.top - base.top}px`,
    width: `${flight.textBox.width}px`,
    height: `${flight.textBox.height}px`,
    transformOrigin: `${flight.origin.x - flight.textBox.left}px ${flight.origin.y - flight.textBox.top}px`,
  });
  flight.host.append(ghost);
  const collapse = ghost.animate(
    [
      { transform: 'scale(1)', opacity: 1, filter: 'none' },
      { transform: 'scale(0.03)', opacity: 0.5, filter: 'blur(0.5px)' },
    ],
    { duration: half, easing: 'cubic-bezier(0.5, 0, 0.9, 0.6)', fill: 'forwards' },
  );
  const fly = () => {
    ghost.remove();
    if (!flight.target) return flight.onArrive();
    const packet = document.createElement('div');
    packet.className = 'af-packet';
    flight.host.append(packet);
    const from = { x: flight.origin.x - base.left, y: flight.origin.y - base.top };
    const to = {
      x: flight.target.left + flight.target.width / 2 - base.left,
      y: flight.target.top + flight.target.height / 2 - base.top,
    };
    const lift = Math.min(from.y, to.y) - Math.max(40, Math.abs(to.x - from.x) * 0.25);
    const keyframes = arc(from, to, lift).map((point, index) => ({
      transform: `translate(${point.x - 4}px, ${point.y - 4}px) scale(${1 + Math.sin((index / ARC_STEPS) * Math.PI) * 0.6})`,
    }));
    const flightAnimation = packet.animate(keyframes, { duration: half, easing: 'linear' });
    const land = () => {
      packet.remove();
      flight.onArrive();
    };
    flightAnimation.finished.then(land, land);
  };
  collapse.finished.then(fly, fly);
}

/** Points on a quadratic curve from `from` to `to` whose peak is around `peakY`. */
export function arc(
  from: { x: number; y: number },
  to: { x: number; y: number },
  peakY: number,
): { x: number; y: number }[] {
  const control = { x: (from.x + to.x) / 2, y: 2 * peakY - (from.y + to.y) / 2 };
  return Array.from({ length: ARC_STEPS + 1 }, (_, step) => {
    const t = step / ARC_STEPS;
    const u = 1 - t;
    return {
      x: u * u * from.x + 2 * u * t * control.x + t * t * to.x,
      y: u * u * from.y + 2 * u * t * control.y + t * t * to.y,
    };
  });
}
