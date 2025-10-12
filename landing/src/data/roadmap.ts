export const CURRENT_VERSION = '0.3.0';
export const IN_PROGRESS_VERSION = '0.4.0';
export const ROADMAP_UPDATED = '2026-06-05';

export type ItemKind = 'feature' | 'perf' | 'infra' | 'docs';

export interface RoadmapItem {
  title: string;
  kind: ItemKind;
  issue?: number;
  description?: string;
  note?: string;
  versionTarget?: string;
}

export interface ShippedRelease {
  version: string;
  date: string;
  highlights: string[];
}

export const roadmapNow: RoadmapItem[] = [
  {
    title: 'Large-corpus benchmark suite',
    kind: 'perf',
    versionTarget: '0.4.0',
    description: 'Extend criterion benchmarks to 500k-row datasets; profile OR fanout, CONTAINS scans, and json_extract() call overhead.',
  },
  {
    title: 'Query latency improvements',
    kind: 'perf',
    versionTarget: '0.4.0',
    description: 'Target sub-10ms for 25%-match queries at 500k rows on indexed fields. Profile and reduce allocations in the executor hot path.',
  },
];

export const roadmapNext: RoadmapItem[] = [
  {
    title: 'Structured output formats: yaml, csv',
    kind: 'feature',
    description: 'Add --output yaml and --output csv to logdive query for pipeline-friendly output.',
