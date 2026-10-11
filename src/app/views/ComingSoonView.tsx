import { PixelIcon, type PixelArt } from '../../shared/ui/PixelIcon';

/** Friendly placeholder for sidebar views that arrive in later phases. */
export function ComingSoonView(props: { icon: PixelArt; text: string }) {
  return (
    <div class="af-view af-coming-soon">
      <PixelIcon art={props.icon} scale={4} />
      <p>{props.text}</p>
    </div>
  );
}
