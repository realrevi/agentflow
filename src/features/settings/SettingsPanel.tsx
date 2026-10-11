// Lazy-loaded settings view (see layout/renderers.tsx). Default export for `lazy()`.
import './settings.css';
import { AppearanceSection } from './sections/AppearanceSection';
import { FilesSection } from './sections/FilesSection';
import { LanguageSection } from './sections/LanguageSection';
import { LayoutSection } from './sections/LayoutSection';
import { TerminalSection } from './sections/TerminalSection';

export default function SettingsPanel() {
  return (
    <div class="af-settings">
      <AppearanceSection />
      <TerminalSection />
      <FilesSection />
      <LanguageSection />
      <LayoutSection />
    </div>
  );
}
