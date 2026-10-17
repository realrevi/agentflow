// Agent installer (phase 8): the lightweight part. The wizard panel loads lazily via the
// layout; the community library (phase 9) will feed templates into the same wizard.
export { agentProfiles, refreshProfiles, watchProfiles } from './profilesStore';
export type { InstallerHooks } from './InstallerPanel';
