import { render } from '@testing-library/svelte/svelte5';
import { afterEach, describe, expect, it } from 'vitest';
import StreamDialog from './StreamDialog.svelte';
import { selectedStream, type StreamMessage } from '../stores';

function message(partial: Partial<StreamMessage>): StreamMessage {
  return {
    is_client: true,
    data: Array.from('payload', (c) => c.charCodeAt(0)),
    timestamp: 1_600_000_000_000_000_000,
    missing_before: 0,
    ...partial,
  };
}

describe('StreamDialog', () => {
  afterEach(() => selectedStream.set(null));

  it('renders the reassembled data of each side', () => {
    selectedStream.set([message({ data: Array.from('GET / HTTP/1.1', (c) => c.charCodeAt(0)) })]);

    const { container } = render(StreamDialog);

    expect(container.textContent).toContain('GET / HTTP/1.1');
    expect(container.textContent).toContain('Total Messages: 1');
  });

  it('does not mention missing bytes when the stream is contiguous', () => {
    selectedStream.set([message({ missing_before: 0 })]);

    const { container } = render(StreamDialog);

    expect(container.textContent).not.toContain('never captured');
  });

  it('reports the bytes of the stream that were never captured', () => {
    // What the reassembler emits when a hole splits a direction in two.
    selectedStream.set([
      message({ is_client: false, data: Array.from('hello', (c) => c.charCodeAt(0)) }),
      message({
        is_client: false,
        data: Array.from('world', (c) => c.charCodeAt(0)),
        missing_before: 92,
      }),
    ]);

    const { container } = render(StreamDialog);

    expect(container.textContent).toContain('92 bytes of this stream were never captured');
    expect(container.textContent).toContain('world');
  });
});
