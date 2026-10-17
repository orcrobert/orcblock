<script lang="ts"></script>

<!-- Placeholder art: the orc standing in the path with his club. -->
<svg viewBox="0 0 200 200" xmlns="http://www.w3.org/2000/svg">
  <ellipse cx="100" cy="186" rx="58" ry="8" fill="var(--orc-shadow)" />

  <g class="sway">
    <!-- legs and feet -->
    <rect x="76" y="140" width="18" height="38" rx="7" fill="var(--orc-cloth-dark)" />
    <rect x="106" y="140" width="18" height="38" rx="7" fill="var(--orc-cloth-dark)" />
    <ellipse cx="84" cy="180" rx="14" ry="7" fill="var(--orc-ink)" />
    <ellipse cx="116" cy="180" rx="14" ry="7" fill="var(--orc-ink)" />

    <!-- torso -->
    <path d="M60 96 Q100 82 140 96 L132 150 Q100 158 68 150 Z" fill="var(--orc-skin)" />
    <rect x="66" y="128" width="68" height="14" rx="4" fill="var(--orc-cloth)" />
    <rect x="94" y="127" width="12" height="16" rx="2" fill="var(--orc-metal)" />
    <path d="M70 142 L130 142 L124 162 L76 162 Z" fill="var(--orc-cloth-dark)" />

    <!-- left arm resting on the hip -->
    <path
      d="M62 100 Q44 118 60 136"
      stroke="var(--orc-skin)"
      stroke-width="16"
      stroke-linecap="round"
      fill="none"
    />
    <ellipse cx="64" cy="98" rx="16" ry="11" fill="var(--orc-metal)" />

    <!-- club held upright in the right hand -->
    <g>
      <rect
        x="146"
        y="58"
        width="12"
        height="96"
        rx="6"
        fill="var(--orc-wood)"
        transform="rotate(10 152 106)"
      />
      <ellipse
        cx="160"
        cy="58"
        rx="16"
        ry="22"
        fill="var(--orc-wood)"
        transform="rotate(10 160 58)"
      />
      <path
        d="M150 44 l-8 -4 6 8 Z M172 50 l8 -4 -6 8 Z M164 76 l8 2 -8 4 Z"
        fill="var(--orc-metal)"
      />
    </g>
    <path
      d="M138 100 Q156 112 150 128"
      stroke="var(--orc-skin)"
      stroke-width="16"
      stroke-linecap="round"
      fill="none"
    />
    <circle cx="150" cy="128" r="9" fill="var(--orc-skin-shade)" />
    <ellipse cx="136" cy="98" rx="16" ry="11" fill="var(--orc-metal)" />

    <!-- head -->
    <g>
      <path d="M72 58 L52 46 L76 50 Z" fill="var(--orc-skin-shade)" />
      <path d="M128 58 L148 46 L124 50 Z" fill="var(--orc-skin-shade)" />
      <path
        d="M72 50 Q72 26 100 26 Q128 26 128 50 L128 66 Q128 90 100 92 Q72 90 72 66 Z"
        fill="var(--orc-skin)"
      />
      <path
        d="M80 52 Q88 46 96 54"
        stroke="var(--orc-ink)"
        stroke-width="4"
        stroke-linecap="round"
        fill="none"
      />
      <path
        d="M120 52 Q112 46 104 54"
        stroke="var(--orc-ink)"
        stroke-width="4"
        stroke-linecap="round"
        fill="none"
      />
      <g class="eyes" fill="var(--orc-ink)">
        <circle cx="89" cy="60" r="3.5" />
        <circle cx="111" cy="60" r="3.5" />
      </g>
      <path
        d="M86 78 Q100 84 114 78"
        stroke="var(--orc-ink)"
        stroke-width="3.5"
        stroke-linecap="round"
        fill="none"
      />
      <path d="M89 80 L87 69 L94 79 Z" fill="var(--orc-tusk)" />
      <path d="M111 80 L113 69 L106 79 Z" fill="var(--orc-tusk)" />
    </g>
  </g>
</svg>

<style>
  .sway {
    transform-origin: 100px 180px;
    animation: sway 5s ease-in-out infinite;
  }
  @keyframes sway {
    0%,
    100% {
      transform: rotate(-1.2deg);
    }
    50% {
      transform: rotate(1.2deg);
    }
  }
  .eyes {
    transform-origin: 100px 60px;
    animation: blink 6s infinite;
  }
  @keyframes blink {
    0%,
    95%,
    100% {
      transform: scaleY(1);
    }
    97% {
      transform: scaleY(0.1);
    }
  }
</style>
