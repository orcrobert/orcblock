<script lang="ts">
  // Material 3 slider over a native range input, so keyboard and screen
  // readers work as usual.
  let {
    value = $bindable(0),
    min,
    max,
    label,
    valuetext,
  }: { value?: number; min: number; max: number; label: string; valuetext: string } = $props();

  const fraction = $derived(max > min ? (value - min) / (max - min) : 0);
</script>

<input
  type="range"
  bind:value
  {min}
  {max}
  step="1"
  aria-label={label}
  aria-valuetext={valuetext}
  style:--fraction={fraction}
/>

<style>
  input {
    --thumb: 20px;
    width: 100%;
    height: 40px;
    margin: 0;
    background: transparent;
    cursor: pointer;
    -webkit-appearance: none;
    appearance: none;
  }
  input:focus-visible {
    outline: none;
  }
  input::-webkit-slider-runnable-track {
    height: 4px;
    border-radius: var(--md-sys-shape-corner-full);
    background: linear-gradient(
      to right,
      var(--md-sys-color-primary) 0 calc(var(--fraction) * 100%),
      var(--md-sys-color-secondary-container) calc(var(--fraction) * 100%) 100%
    );
  }
  input::-webkit-slider-thumb {
    width: var(--thumb);
    height: var(--thumb);
    margin-top: calc((4px - var(--thumb)) / 2);
    border: none;
    border-radius: var(--md-sys-shape-corner-full);
    background: var(--md-sys-color-primary);
    box-shadow: 0 0 0 0 color-mix(in srgb, var(--md-sys-color-primary) 12%, transparent);
    transition: box-shadow var(--md-sys-motion-duration-short) var(--md-sys-motion-easing-standard);
    -webkit-appearance: none;
    appearance: none;
  }
  input:hover::-webkit-slider-thumb {
    box-shadow: 0 0 0 10px color-mix(in srgb, var(--md-sys-color-primary) 8%, transparent);
  }
  input:focus-visible::-webkit-slider-thumb,
  input:active::-webkit-slider-thumb {
    box-shadow: 0 0 0 10px color-mix(in srgb, var(--md-sys-color-primary) 16%, transparent);
  }
</style>
