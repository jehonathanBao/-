import '@testing-library/jest-dom/vitest';
import { cleanup, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import EventTraceTimeline from '../components/EventTraceTimeline.jsx';

afterEach(cleanup);
const base = 1788750000000;
describe('event observation timeline', () => {
  it('shows an honest empty state without inventing events', () => {
    render(<EventTraceTimeline items={[]} />);
    expect(screen.getByRole('region', { name: '事件时间轴' })).toHaveTextContent('暂无可展示的事件');
    expect(screen.queryByRole('button')).not.toBeInTheDocument();
  });
  it('bounds the newest unique events and orders them chronologically without mutating input', () => {
    const items = Array.from({ length: 12 }, (_, i) => ({ id: String(i), ts: base + i * 60000, symbol: 'BTC' })).reverse();
    const saved = JSON.stringify(items);
    render(<EventTraceTimeline items={[...items, items[0], { id: 'bad', ts: Infinity }, { id: 'bad2', ts: 'no date' }]} />);
    const rows = screen.getAllByRole('listitem');
    expect(rows).toHaveLength(8);
    expect(within(rows[0]).getByText('方向未知')).toBeInTheDocument();
    expect(rows[0].querySelector('time')).toHaveAttribute('datetime', new Date(base + 4 * 60000).toISOString());
    expect(JSON.stringify(items)).toBe(saved);
  });
  it('opens the selected original event using keyboard without any network operation', async () => {
    const onSelect = vi.fn();
    const user = userEvent.setup();
    render(<EventTraceTimeline items={[{ id: 'event-a', ts: base, symbol: 'ETH', direction: 'sell' }]} onSelect={onSelect} selectedId='event-a' />);
    const button = screen.getByRole('button', { name: /查看 ETH/ });
    expect(button).toHaveAttribute('aria-pressed', 'true');
    button.focus();
    await user.keyboard('{Enter}');
    expect(onSelect).toHaveBeenCalledWith('event-a');
    expect(onSelect).toHaveBeenCalledTimes(1);
  });
});
