// Rust sends raw pty bytes in arbitrary chunks, so a multi-byte UTF-8 character can be
// split between two messages. A streaming TextDecoder keeps the partial bytes until
// the rest arrives.
export class OutputDecoder {
  private readonly decoder = new TextDecoder('utf-8');

  decode(chunk: Uint8Array): string {
    return this.decoder.decode(chunk, { stream: true });
  }

  /** Ends the stream; an unfinished character becomes U+FFFD. */
  flush(): string {
    return this.decoder.decode();
  }
}
