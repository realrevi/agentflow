// Lazy-loaded settings view (see layout/renderers.tsx). Default export for `lazy()`.
import './settings.css';
import { AppearanceSection } from './sections/AppearanceSection';
import { CharactersSection } from './sections/CharactersSection';
import { FilesSection } from './sections/FilesSection';
import { LanguageSection } from './sections/LanguageSection';
import { LayoutSection } from './sections/LayoutSection';
import { ProvidersSection } from './sections/ProvidersSection';
import { TerminalSection } from './sections/TerminalSection';
import { UsageSection } from './sections/UsageSection';

export default function SettingsPanel() {
  return (
    <div class="af-settings">
      <AppearanceSection />
      <TerminalSection />
      <CharactersSection />
      <ProvidersSection />
      <UsageSection />
      <FilesSection />
      <LanguageSection />
      <LayoutSection />
    </div>
  );
}
