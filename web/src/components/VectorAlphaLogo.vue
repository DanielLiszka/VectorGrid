<script setup lang="ts">
import { computed } from 'vue'
import { useMediaQuery } from '@vueuse/core'
import animatedLogoUrl from '../assets/branding/vectoralpha-animated.svg'
import staticLogoUrl from '../assets/branding/vectoralpha-static.svg'

const props = withDefaults(
  defineProps<{
    alt?: string
    decorative?: boolean
    hoverAnimate?: boolean
    size?: number | string
  }>(),
  {
    alt: 'VectorAlpha logo',
    decorative: false,
    hoverAnimate: false,
    size: 22,
  },
)

const sizeValue = computed(() => (typeof props.size === 'number' ? `${props.size}px` : String(props.size || '22px')))
const staticAlt = computed(() => (props.decorative ? '' : props.alt))
const prefersReducedMotion = useMediaQuery('(prefers-reduced-motion: reduce)')
const animateOnHover = computed(() => props.hoverAnimate && !prefersReducedMotion.value)
</script>

<template>
  <span
    class="vectorAlphaLogo"
    :class="{ hoverAnimate: animateOnHover }"
    :style="{ '--va-logo-size': sizeValue }"
    :aria-hidden="decorative ? 'true' : undefined"
  >
    <img class="vectorAlphaLogoImg vectorAlphaLogoStatic" :src="staticLogoUrl" :alt="staticAlt" draggable="false" />
    <img
      v-if="animateOnHover"
      class="vectorAlphaLogoImg vectorAlphaLogoAnimated"
      :src="animatedLogoUrl"
      alt=""
      aria-hidden="true"
      draggable="false"
    />
  </span>
</template>

<style scoped>
.vectorAlphaLogo {
  position: relative;
  display: inline-grid;
  place-items: center;
  width: var(--va-logo-size);
  aspect-ratio: 6 / 5;
  flex: 0 0 auto;
  line-height: 0;
}

.vectorAlphaLogoImg {
  width: 100%;
  height: 100%;
  display: block;
  object-fit: contain;
  user-select: none;
  -webkit-user-drag: none;
}

.vectorAlphaLogoAnimated {
  position: absolute;
  inset: 0;
  opacity: 0;
  transition: opacity var(--motion-ui, 160ms) var(--ease-standard, cubic-bezier(0.2, 0, 0, 1));
}

.vectorAlphaLogo.hoverAnimate:hover .vectorAlphaLogoAnimated {
  opacity: 1;
}

.vectorAlphaLogo.hoverAnimate:hover .vectorAlphaLogoStatic {
  opacity: 0;
}

@media (prefers-reduced-motion: reduce) {
  .vectorAlphaLogoAnimated,
  .vectorAlphaLogoStatic {
    transition: none;
  }
}
</style>
