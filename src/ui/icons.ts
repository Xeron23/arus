const paths: Record<string, string> = {
  home: "M3 10 12 3l9 7v10a1 1 0 0 1-1 1h-5v-7H9v7H4a1 1 0 0 1-1-1z",
  activity: "M3 7h18M3 12h12M3 17h8m6-2 3 3 3-3m-3 3v-7",
  wallet:
    "M20 7V5a2 2 0 0 0-2-2H5a3 3 0 0 0 0 6h15v12H5a2 2 0 0 1-2-2V6m17 7h-5v4h5m-3-2h.01",
  budget: "M12 3v9h9A9 9 0 1 1 12 3zm4-1a9 9 0 0 1 6 6h-6z",
  more: "M5 12h.01M12 12h.01M19 12h.01",
  plus: "M12 5v14M5 12h14",
  sync: "M20 7v5h-5M4 17v-5h5m-4-4a8 8 0 0 1 13-3l2 3M4 16l2 3a8 8 0 0 0 13-3",
  up: "M7 17 17 7M7 7h10v10",
  down: "M7 7l10 10M7 17h10V7",
  chevron: "m9 5 7 7-7 7",
  back: "m15 5-7 7 7 7",
  close: "m6 6 12 12M6 18 18 6",
  eye: "M2 12s4-7 10-7 10 7 10 7-4 7-10 7S2 12 2 12zm10-3a3 3 0 1 0 .01 0",
  mail: "M3 5h18v14H3zM3 5l9 8 9-8",
  bell: "M18 8a6 6 0 0 0-12 0c0 7-3 7-3 9h18c0-2-3-2-3-9m-11 12h4",
  shield: "m12 2 9 4v6c0 6-9 10-9 10S3 18 3 12V6zM8 12l3 3 5-6",
  check: "m5 12 4 4L19 6",
  search: "M10 3a7 7 0 1 0 0 14 7 7 0 0 0 0-14zm5 12 6 6",
  filter: "M4 6h16M7 12h10m-7 6h4M8 3v6m8 0v6m-4 0v6",
  food: "M5 3v6a3 3 0 0 0 6 0V3M8 3v18M18 3c-4 3-4 8 0 9v9m0-18v9",
  shop: "M4 7h16l-1 14H5zM8 8V6a4 4 0 0 1 8 0v2",
  car: "m5 5-3 7v7h3v-3h14v3h3v-7l-3-7zM2 12h20M6 13v1m12-1v1",
  house: "M3 10 12 3l9 7v11H3zM9 21v-7h6v7",
  transfer: "M3 7h17l-4-4m4 4-4 4M21 17H4l4-4m-4 4 4 4",
  download: "M12 3v12m-5-5 5 5 5-5M4 16v5h16v-5",
  upload: "M12 17V3m-5 5 5-5 5 5M4 16v5h16v-5",
  lock: "M6 10h12v11H6zM8 10V6a4 4 0 0 1 8 0v4m-4 5v2",
  fingerprint:
    "M4 12a8 8 0 0 1 16 0v4M8 20c2-3 0-5 0-8a4 4 0 0 1 8 0v6m-4-6v5c0 3-1 5-2 6m-6-8v3m12 4 2-3",
  info: "M12 17v-6m0-4h.01M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0",
  clock: "M12 7v5l3 2M22 12a10 10 0 1 1-20 0 10 10 0 0 1 20 0",
  edit: "m4 16 12-12 4 4L8 20H4zm10-10 4 4",
  trash: "M3 6h18M9 6V3h6v3M5 6l1 15h12l1-15M10 10v7m4-7v7",
  receipt: "M5 3h14v18l-4-2-3 2-3-2-4 2zM8 7h8m-8 4h8m-8 4h5",
  chart: "M3 21h18M6 17V9m6 8V3m6 14v-6",
  spark: "m12 2 2.5 7.5L22 12l-7.5 2.5L12 22l-2.5-7.5L2 12l7.5-2.5z",
  coffee:
    "M4 7h13v9a4 4 0 0 1-4 4H8a4 4 0 0 1-4-4zm13 0h2a3 3 0 0 1 0 6h-2M7 2v2m6-2v2",
  arrow: "M5 12h14m-6-6 6 6-6 6",
  logout: "M9 3H4v18h5m5-14 5 5-5 5m-6-5h11",
  bank: "m2 8 10-6 10 6H2zm2 12h16M6 10v8m6-8v8m6-8v8",
  cloud: "M5 17a5 5 0 1 1 1-10 6 6 0 0 1 12 2 4 4 0 0 1 0 8m-13 4L20 2",
};
export const icon = (name: string) =>
  `<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.65" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="${paths[name] || paths.receipt}"/></svg>`;
export const logo = () =>
  `<svg viewBox="0 0 30 30" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round"><path d="M5 19c5-13 7 5 12-8m-5 11c5-13 7 5 12-8M5 11c3-7 4 1 7-5"/></svg>`;

