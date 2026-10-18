// Lazy-loaded settings view (see layout/renderers.tsx). Default export for `lazy()`.
import './settings.css';
import { AboutSection } from './sections/AboutSection';
import { AppearanceSection } from './sections/AppearanceSection';
import { CharactersSection } from './sections/CharactersSection';
import { FilesSection } from './sections/FilesSection';
import { LanguageSection } from './sections/LanguageSection';
import { LayoutSection } from './sections/LayoutSection';
import { AgentsSection } from './sections/AgentsSection';
import { ProvidersSection } from './sections/ProvidersSection';
import { TerminalSection } from './sections/TerminalSection';
import { UsageSection } from './sections/UsageSection';
import { WorkstreamsSection } from './sections/WorkstreamsSection';

export default function SettingsPanel() {
  return (
    <div class="af-settings">
      <AppearanceSection />
      <TerminalSection />
      <CharactersSection />
      <ProvidersSection />
      <AgentsSection />
      <WorkstreamsSection />
      <UsageSection />
      <FilesSection />
      <LanguageSection />
      <LayoutSection />
      <AboutSection />
    </div>
  );
}
