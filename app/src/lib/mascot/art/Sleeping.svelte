<script lang="ts"></script>

<!-- Placeholder art: the orc curled up asleep across the path. -->
<svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
  <ellipse cx="104" cy="168" rx="78" ry="9" fill="var(--orc-shadow)" />

  <!-- club lying behind him -->
  <g transform="rotate(-8 150 150)">
    <rect x="118" y="146" width="66" height="10" rx="5" fill="var(--orc-wood)" />
    <ellipse cx="180" cy="151" rx="14" ry="11" fill="var(--orc-wood)" />
  </g>

  <g class="breathe">
    <!-- body -->
    <path
      d="M52 160 Q44 118 92 112 Q150 104 164 140 Q170 162 150 166 L64 166 Q54 166 52 160 Z"
      fill="var(--orc-skin)"
    />
    <path d="M88 136 Q120 128 152 140 L152 160 Q120 150 88 158 Z" fill="var(--orc-cloth)" />
    <ellipse cx="96" cy="114" rx="20" ry="12" fill="var(--orc-metal)" />
    <!-- arm under the head -->
    <path
      d="M40 150 Q60 140 86 150"
      stroke="var(--orc-skin-shade)"
      stroke-width="14"
      stroke-linecap="round"
      fill="none"
    />

    <!-- head -->
    <g>
      <path d="M30 118 L14 104 L36 110 Z" fill="var(--orc-skin-shade)" />
      <ellipse cx="50" cy="128" rx="26" ry="24" fill="var(--orc-skin)" />
      <path
        d="M34 120 Q40 116 46 120"
        stroke="var(--orc-ink)"
        stroke-width="3"
        stroke-linecap="round"
        fill="none"
      />
      <path
        d="M56 120 Q62 116 68 120"
        stroke="var(--orc-ink)"
        stroke-width="3"
        stroke-linecap="round"
        fill="none"
      />
      <path
        d="M40 140 Q50 144 62 140"
        stroke="var(--orc-ink)"
        stroke-width="3"
        stroke-linecap="round"
        fill="none"
      />
      <path d="M43 141 L41 132 L47 140 Z" fill="var(--orc-tusk)" />
      <path d="M59 141 L61 132 L55 140 Z" fill="var(--orc-tusk)" />
    </g>
  </g>

  <g class="z" fill="var(--orc-z)" font-family="var(--font)" font-weight="700">
    <text class="z1" x="70" y="96" font-size="14">z</text>
    <text class="z2" x="82" y="80" font-size="18">z</text>
    <text class="z3" x="96" y="60" font-size="22">Z</text>
  </g>
</svg>

<style>
  .breathe {
    transform-origin: 100px 166px;
    animation: breathe 4.5s ease-in-out infinite;
  }
  @keyframes breathe {
    0%,
    100% {
      transform: scale(1, 1);
    }
    50% {
      transform: scale(1.015, 1.04);
    }
  }
  .z text {
    opacity: 0;
    animation: drift 4.5s ease-out infinite;
  }
  .z .z2 {
    animation-delay: 1.5s;
  }
  .z .z3 {
    animation-delay: 3s;
  }
  @keyframes drift {
    0% {
      opacity: 0;
      transform: translate(0, 6px);
    }
    25% {
      opacity: 0.9;
    }
    100% {
      opacity: 0;
      transform: translate(10px, -10px);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .z text {
      opacity: 0.8;
    }
  }
</style>
