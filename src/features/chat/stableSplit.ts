// While a reply streams, only its last block changes. Rendering the finished blocks once
// and re-rendering just the tail keeps each frame cheap, however long the reply gets.

/** `head` = complete blocks (ends after a blank line outside a code fence); `tail` = the rest. */
export function splitStable(text: string): { head: string; tail: string } {
  let fence: string | null = null;
  let boundary = 0;
  let offset = 0;
  for (const line of text.split('\n')) {
    const end = offset + line.length + 1;
    const marker = /^ {0,3}(`{3,}|~{3,})/.exec(line)?.[1];
    if (marker) {
      if (fence === null) fence = marker;
      else if (
        marker[0] === fence[0] &&
        marker.length >= fence.length &&
        !line.trim().slice(marker.length)
      ) {
        fence = null;
      }
    } else if (fence === null && line.trim() === '' && end <= text.length) {
      boundary = end;
    }
    offset = end;
  }
  return { head: text.slice(0, boundary), tail: text.slice(boundary) };
}
