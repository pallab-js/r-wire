<script lang="ts">
  import { onMount } from 'svelte';

  export let onComplete: () => void;

  let currentStep = 0;

  const steps = [
    {
      title: 'Welcome to AuraCap',
      description:
        'A professional network packet analyzer for macOS. Start capturing packets with ease.',
    },
    {
      title: 'Capture Traffic',
      description:
        'Select an interface and click Start Capture to begin. Use BPF filters to focus on specific traffic.',
    },
    {
      title: 'Analyze Packets',
      description:
        'Click any packet to view details. Use Essentials for overview, Protocol Tree for breakdown, Hex View for raw bytes.',
    },
  ];

  function next() {
    if (currentStep < steps.length - 1) {
      currentStep++;
    } else {
      complete();
    }
  }

  function previous() {
    if (currentStep > 0) {
      currentStep--;
    }
  }

  function complete() {
    localStorage.setItem('auracap_onboarding_complete', 'true');
    onComplete();
  }

  function skip() {
    complete();
  }

  onMount(() => {
    const completed = localStorage.getItem('auracap_onboarding_complete');
    if (completed === 'true') {
      onComplete();
    }
  });

  $: progress = ((currentStep + 1) / steps.length) * 100;
</script>

<div
  class="fixed inset-0 z-50 flex items-center justify-center"
  style="background-color: rgba(0, 0, 0, 0.6);"
>
  <div class="w-full max-w-lg rounded-lg overflow-hidden" style="background-color: var(--bg-page);">
    <!-- Progress bar -->
    <div class="h-1" style="background-color: var(--border-standard);">
      <div
        class="h-full transition-all duration-300"
        style="width: {progress}%; background-color: var(--brand-green);"
      ></div>
    </div>

    <div class="p-8">
      <!-- Step number -->
      <div class="flex items-center justify-center mb-6">
        <div
          class="w-16 h-16 rounded-full flex items-center justify-center text-2xl font-medium"
          style="background-color: var(--border-standard); color: var(--brand-green);"
        >
          {currentStep + 1}
        </div>
      </div>

      <!-- Step content -->
      <div class="text-center mb-8">
        <h2 class="text-xl font-medium mb-2" style="color: var(--text-primary);">
          {steps[currentStep].title}
        </h2>
        <p class="text-sm leading-relaxed" style="color: var(--text-secondary);">
          {steps[currentStep].description}
        </p>
      </div>

      <!-- Step indicators -->
      <div class="flex justify-center gap-2 mb-6">
        {#each steps as _step, i}
          <button
            class="w-2 h-2 rounded-full transition-all cursor-pointer"
            style="background-color: {i === currentStep
              ? 'var(--brand-green)'
              : 'var(--border-prominent)'}; border: none;"
            on:click={() => (currentStep = i)}
            aria-label="Go to step {i + 1}"
          ></button>
        {/each}
      </div>

      <!-- Actions -->
      <div class="flex items-center justify-between">
        <button
          class="px-4 py-2 text-sm font-medium transition-colors cursor-pointer"
          style="color: var(--text-muted); background: transparent; border: none;"
          on:click={skip}
        >
          Skip Tour
        </button>

        <div class="flex gap-3">
          {#if currentStep > 0}
            <button
              class="px-4 py-2 text-sm font-medium rounded cursor-pointer transition-colors"
              style="background-color: var(--border-standard); color: var(--text-primary); border: 1px solid var(--border-standard);"
              on:click={previous}
            >
              Back
            </button>
          {/if}
          <button
            class="px-6 py-2 text-sm font-medium rounded cursor-pointer transition-colors"
            style="background-color: var(--brand-green); color: var(--bg-button); border: none;"
            on:click={next}
          >
            {currentStep === steps.length - 1 ? 'Get Started' : 'Next'}
          </button>
        </div>
      </div>
    </div>
  </div>
</div>
