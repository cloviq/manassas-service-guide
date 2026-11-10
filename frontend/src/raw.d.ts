// Static document imports used by the source-content regression tests.
declare module '*.html?raw' {
  const content: string;
  export default content;
}
