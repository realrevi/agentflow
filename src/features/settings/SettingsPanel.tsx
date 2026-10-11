// Lazy-loaded settings view (see layout/renderers.tsx). Default export for `lazy()`.
import './settings.css';
import { AppearanceSection } from './sections/AppearanceSection';
import { LanguageSection } from './sections/LanguageSection';
import { LayoutSection } from './sections/LayoutSection';

export default function SettingsPanel() {
  return (
    <div class="af-settings">
      <AppearanceSection />
      <LanguageSection />
      <LayoutSection />
    </div>
  );
}
