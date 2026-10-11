// Files attached to a prompt: read through the workspace fs (text only), capped at 200 KB in
// total like the backend, each with a rough token estimate (real counting is phase 6).
import { AppError } from '../../shared/errors';
import { ipc } from '../../shared/ipc';
import type { Attachment } from '../../shared/types';
import { t } from '../i18n';

export const MAX_ATTACHMENT_BYTES = 200 * 1024;

const encoder = new TextEncoder();

export function byteSize(text: string): number {
  return encoder.encode(text).length;
}

/** About 4 characters per token: good enough for a hint. */
export function estimateTokens(text: string): number {
  return text ? Math.max(1, Math.ceil(text.length / 4)) : 0;
}

export function totalBytes(list: readonly Attachment[]): number {
  return list.reduce((sum, item) => sum + byteSize(item.content), 0);
}

/** Adds `next` unless it is already there or the total would pass the cap. */
export function withAttachment(list: readonly Attachment[], next: Attachment): Attachment[] {
  const others = list.filter((item) => item.path !== next.path);
  if (totalBytes(others) + byteSize(next.content) > MAX_ATTACHMENT_BYTES) {
    throw new AppError('tooLarge', t('chat.attach.tooLarge'));
  }
  return [...others, next];
}

export async function readAttachment(path: string): Promise<Attachment> {
  const file = await ipc.fsRead(path);
  if (file.binary || file.content === null) {
    throw new AppError(
      'invalidInput',
      t('chat.attach.binary', { name: path.split('/').pop() ?? path }),
    );
  }
  return { path, content: file.content };
}
