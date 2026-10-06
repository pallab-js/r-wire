import { render } from '@testing-library/svelte/svelte5';
import { get } from 'svelte/store';
import type { Mock } from 'vitest';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { tick } from 'svelte';
import { invoke } from '@tauri-apps/api/tauri';
import PacketList from './PacketList.svelte';
import {
  displayFilter,
  selectedPacket,
  totalFilteredCount,
  type PacketDetail,
  type PacketSummary,
} from '../stores';

// The global setup mocks this module; widen it so the test can install its own
// per-command behaviour without fighting the generic signature.
const invokeMock = invoke as unknown as Mock;
const ROW_HEIGHT = 28;
const TOTAL_PACKETS = 1000;

// jsdom has no layout engine, so `bind:clientHeight` needs the constructor to
// exist even though it never fires.
if (typeof globalThis.ResizeObserver === 'undefined') {
  class StubResizeObserver {
    observe() {}
    unobserve() {}
    disconnect() {}
  }
  (globalThis as { ResizeObserver?: unknown }).ResizeObserver = StubResizeObserver;
}

let rejectFetchesBeyondZero = false;

function makeRow(id: number): PacketSummary {
  return {
    id,
    timestamp: id * 1_000_000,
    source_addr: '10.0.0.1',
    dest_addr: '10.0.0.2',
    protocol: 'TCP',
    length: 60,
    info: 'packet',
    src_port: 1234,
    dst_port: 80,
  };
}

describe('PacketList windowed fetching', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    vi.clearAllMocks();
    rejectFetchesBeyondZero = false;
    totalFilteredCount.set(0);
    displayFilter.set('');

    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_packet_count') {
        return Promise.resolve(TOTAL_PACKETS);
      }
      if (cmd === 'get_packets') {
        const offset = (args?.offset as number) ?? 0;
        const limit = (args?.limit as number) ?? 0;
        if (rejectFetchesBeyondZero && offset > 0) {
          return Promise.reject(new Error('database unavailable'));
        }
        return Promise.resolve(Array.from({ length: limit }, (_, i) => makeRow(offset + i + 1)));
      }
      return Promise.resolve(null);
    });
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  async function mountAndSettle() {
    const result = render(PacketList);
    await vi.runAllTimersAsync();
    await tick();
    return result;
  }

  function fetchCalls(): { args: { offset?: number; limit?: number } }[] {
    return invokeMock.mock.calls
      .filter((call) => call[0] === 'get_packets')
      .map((call) => ({ args: call[1] as { offset?: number; limit?: number } }));
  }

  function scrollTo(row: number) {
    const scroller = document.querySelector<HTMLElement>('.overflow-y-auto');
    expect(scroller).not.toBeNull();
    Object.defineProperty(scroller, 'scrollTop', {
      value: row * ROW_HEIGHT,
      configurable: true,
      writable: true,
    });
    scroller!.dispatchEvent(new Event('scroll'));
  }

  /** Packet ids of the rendered data rows, in document order. */
  function renderedIds(container: HTMLElement): number[] {
    return [...container.querySelectorAll('tbody tr')]
      .filter((tr) => !tr.querySelector('td[colspan]'))
      .map((tr) => Number(tr.querySelector('td')?.textContent?.trim()))
      .filter((id) => !Number.isNaN(id));
  }

  /** Height of the spacer row *above* the data rows (0 when there is none). */
  function topSpacerHeight(container: HTMLElement): number {
    const firstRow = container.querySelector<HTMLElement>('tbody tr');
    if (!firstRow || !firstRow.querySelector('td[colspan]')) return 0;
    return parseInt(firstRow.style.height, 10) || 0;
  }

  it('renders the window it fetched, with no spacer at offset 0', async () => {
    const { container } = await mountAndSettle();

    const calls = fetchCalls();
    expect(calls).toHaveLength(1);
    expect(calls[0].args.offset).toBe(0);
    expect(calls[0].args.limit).toBeGreaterThan(0);

    const ids = renderedIds(container);
    expect(ids[0]).toBe(1);
    expect(ids).toHaveLength(calls[0].args.limit!);
    expect(topSpacerHeight(container)).toBe(0);
  });

  it('keeps rows at their fetched offset while a new fetch is in flight', async () => {
    const { container } = await mountAndSettle();
    expect(renderedIds(container)[0]).toBe(1);

    scrollTo(500);
    await tick();

    // Regression: the spacer used to jump to the *requested* offset while the
    // rows below it were still the ones fetched for offset 0, so every row was
    // drawn against a scroll position that did not match its packet id.
    expect(fetchCalls()).toHaveLength(1);
    expect(topSpacerHeight(container)).toBe(0);
    expect(renderedIds(container)[0]).toBe(1);

    await vi.runAllTimersAsync();
    await tick();

    expect(fetchCalls()).toHaveLength(2);
    const loadedOffset = fetchCalls()[1].args.offset!;
    expect(renderedIds(container)[0]).toBe(loadedOffset + 1);
    expect(topSpacerHeight(container)).toBe(loadedOffset * ROW_HEIGHT);
  });

  it('coalesces a scroll burst into a single query', async () => {
    await mountAndSettle();
    expect(fetchCalls()).toHaveLength(1);

    for (let row = 501; row <= 520; row += 1) {
      scrollTo(row);
      await tick();
    }

    await vi.runAllTimersAsync();
    await tick();

    // One query for the settled window, not one per scroll event.
    expect(fetchCalls()).toHaveLength(2);
  });

  it('keeps the rendered rows when a fetch fails', async () => {
    rejectFetchesBeyondZero = true;
    const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});
    const { container } = await mountAndSettle();
    const before = renderedIds(container);
    expect(before).toHaveLength(fetchCalls()[0].args.limit!);

    scrollTo(500);
    await tick();
    await vi.runAllTimersAsync();
    await tick();

    expect(errorSpy).toHaveBeenCalledWith('Failed to fetch packets:', expect.any(Error));
    // The rows stay where they were fetched for — no blank list, no rows
    // redrawn against a different scroll offset.
    expect(renderedIds(container)).toEqual(before);
    expect(topSpacerHeight(container)).toBe(0);
  });
});

describe('PacketList response ordering', () => {
  beforeEach(() => {
    vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
    vi.clearAllMocks();
    totalFilteredCount.set(0);
    displayFilter.set('');
    selectedPacket.set(null);
  });

  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  function dataRows(container: HTMLElement): HTMLElement[] {
    return [...container.querySelectorAll<HTMLElement>('tbody tr')].filter(
      (tr) => !tr.querySelector('td[colspan]'),
    );
  }

  /** Flushes component updates *and* queued `invoke` continuations. */
  async function flush() {
    await tick();
    await tick();
  }

  function makeDetail(id: number): PacketDetail {
    return {
      summary: makeRow(id),
      layers: [],
      raw_bytes: [],
      expert_summary: [],
      narrative: { summary: '', technical_details: [] },
      intelligence: { entropy: 0, manufacturer: null, risk_score: 0 },
      artifacts: [],
    };
  }

  it('ignores a count that lands after a newer filter was queried', async () => {
    // Hold every count response so the test controls the order they resolve in.
    const pendingCounts: ((value: number) => void)[] = [];
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_packet_count') {
        return new Promise<number>((resolve) => pendingCounts.push(resolve));
      }
      if (cmd === 'get_packets') {
        const offset = (args?.offset as number) ?? 0;
        const limit = (args?.limit as number) ?? 0;
        return Promise.resolve(Array.from({ length: limit }, (_, i) => makeRow(offset + i + 1)));
      }
      return Promise.resolve(null);
    });

    render(PacketList);
    await vi.runAllTimersAsync();
    await tick();
    pendingCounts[0](TOTAL_PACKETS);
    await tick();
    expect(get(totalFilteredCount)).toBe(TOTAL_PACKETS);

    // First filter change: this response is deliberately held open.
    displayFilter.set('tcp');
    await vi.advanceTimersByTimeAsync(300); // 200ms filter debounce
    await tick();
    expect(pendingCounts).toHaveLength(2);

    // Second filter change supersedes it, and answers first.
    displayFilter.set('udp');
    await vi.advanceTimersByTimeAsync(300);
    await tick();
    expect(pendingCounts).toHaveLength(3);
    pendingCounts[2](42);
    await tick();
    expect(get(totalFilteredCount)).toBe(42);

    // The superseded filter's answer finally arrives — it must lose, or the
    // virtual scroller keeps the wrong height for the filter in effect.
    pendingCounts[1](777);
    await tick();
    expect(get(totalFilteredCount)).toBe(42);
  });

  it('never lets an older packet detail overwrite a newer selection', async () => {
    const pendingDetails = new Map<number, (detail: PacketDetail) => void>();
    invokeMock.mockImplementation((cmd: string, args?: Record<string, unknown>) => {
      if (cmd === 'get_packet_detail') {
        const id = args?.id as number;
        return new Promise<PacketDetail>((resolve) => pendingDetails.set(id, resolve));
      }
      if (cmd === 'get_packet_count') return Promise.resolve(TOTAL_PACKETS);
      if (cmd === 'get_packets') {
        const offset = (args?.offset as number) ?? 0;
        const limit = (args?.limit as number) ?? 0;
        return Promise.resolve(Array.from({ length: limit }, (_, i) => makeRow(offset + i + 1)));
      }
      return Promise.resolve(null);
    });

    const { container } = render(PacketList);
    await vi.runAllTimersAsync();
    await tick();

    const rows = dataRows(container);
    expect(rows.length).toBeGreaterThan(1);

    rows[0].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await tick();
    rows[1].dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await tick();

    const firstId = Number(rows[0].querySelector('td')?.textContent?.trim());
    const secondId = Number(rows[1].querySelector('td')?.textContent?.trim());
    expect(pendingDetails.has(firstId)).toBe(true);
    expect(pendingDetails.has(secondId)).toBe(true);

    // The newer click answers first.
    pendingDetails.get(secondId)!(makeDetail(secondId));
    await flush();
    expect(get(selectedPacket)?.summary.id).toBe(secondId);

    // The older click answers afterwards and must be dropped: otherwise the
    // detail pane describes a packet that is no longer the selected row.
    pendingDetails.get(firstId)!(makeDetail(firstId));
    await flush();
    expect(get(selectedPacket)?.summary.id).toBe(secondId);
  });
});
