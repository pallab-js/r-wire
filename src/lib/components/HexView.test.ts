import { render, screen } from '@testing-library/svelte/svelte5';
import { describe, it, expect } from 'vitest';
import HexView from './HexView.svelte';
import { highlightedRange } from '../stores';
import { tick } from 'svelte';

describe('HexView.svelte', () => {
  it('should render hex and ascii data', () => {
    const rawBytes = [72, 101, 108, 108, 111]; // "Hello"
    render(HexView, { rawBytes });

    expect(screen.getByText('48')).toBeInTheDocument(); // H
    expect(screen.getByText('65')).toBeInTheDocument(); // e
    expect(screen.getByText('H')).toBeInTheDocument();
    expect(screen.getByText('e')).toBeInTheDocument();
  });

  it('should highlight bytes when highlightedRange is set', async () => {
    const rawBytes = [72, 101, 108, 108, 111];
    render(HexView, { rawBytes });

    highlightedRange.set([0, 2]); // Highlight "He"
    await tick();

    // The component should have applied highlighting via inline styles using OpenCode design system
    const highlightedByte = screen.getByText('48');
    expect(highlightedByte.getAttribute('style')).toContain('background-color: var(--opencode-blue)');
  });
});
