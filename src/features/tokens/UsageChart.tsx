// The last 30 days as a hand-drawn SVG bar chart, stacked by source, theme colors only.
// Hovering a bar shows its date and the value per source.
import { createMemo, createSignal, For, Show } from 'solid-js';
import type { UsageDay } from '../../shared/types';
import { t } from '../i18n';
import { localDate, shapeChart, type Bar, type ChartMetric } from './chart';
import { formatCost, formatTokens } from './format';
import { SOURCE_COLOR, SOURCE_LABEL } from './labels';

const WIDTH = 300;
const HEIGHT = 96;
const GAP = 2;

export function UsageChart(props: { days: readonly UsageDay[]; metric: ChartMetric }) {
  const chart = createMemo(() => shapeChart(props.days, props.metric, localDate()));
  const [hovered, setHovered] = createSignal<number | null>(null);
  const barWidth = () => WIDTH / chart().bars.length;
  const y = (value: number) => HEIGHT - (value / chart().max) * HEIGHT;
  const format = (value: number) =>
    props.metric === 'cost' ? formatCost(value) : formatTokens(value);
  const hoveredBar = (): Bar | null => {
    const index = hovered();
    return index === null ? null : (chart().bars[index] ?? null);
  };

  return (
    <div class="af-usage-chart" onMouseLeave={() => setHovered(null)}>
      <svg
        viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
        preserveAspectRatio="none"
        role="img"
        aria-label={t('usage.chart.label')}
      >
        <For each={chart().bars}>
          {(bar, index) => (
            <g onMouseEnter={() => setHovered(index())}>
              {/* A full-height hit area, so empty days can be hovered too. */}
              <rect
                class="af-usage-chart-hit"
                x={index() * barWidth()}
                y={0}
                width={barWidth()}
                height={HEIGHT}
              />
              <For each={bar.segments}>
                {(segment) => (
                  <rect
                    x={index() * barWidth() + GAP / 2}
                    y={y(segment.y1)}
                    width={Math.max(1, barWidth() - GAP)}
                    height={Math.max(0.5, y(segment.y0) - y(segment.y1))}
                    fill={SOURCE_COLOR[segment.source]}
                    opacity={hovered() === null || hovered() === index() ? 1 : 0.55}
                  />
                )}
              </For>
            </g>
          )}
        </For>
      </svg>
      <div class="af-usage-chart-axis">
        <span>{chart().bars[0]?.date.slice(5)}</span>
        <span>{t('usage.chart.today')}</span>
      </div>
      <Show when={hoveredBar()}>
        {(bar) => (
          <div class="af-usage-tooltip" role="status">
            <strong>{bar().date}</strong>
            <span>{format(bar().total)}</span>
            <For each={bar().segments}>
              {(segment) => (
                <span class="af-usage-tooltip-row">
                  <i style={{ background: SOURCE_COLOR[segment.source] }} />
                  {t(SOURCE_LABEL[segment.source])}: {format(segment.value)}
                </span>
              )}
            </For>
            <Show when={bar().segments.length === 0}>
              <span class="af-usage-muted">{t('usage.chart.none')}</span>
            </Show>
          </div>
        )}
      </Show>
    </div>
  );
}
