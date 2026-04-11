<script lang="ts">
  import { onMount } from 'svelte';

  export let onComplete: () => void;

  let currentStep = 0;

  const steps = [
    {
      title: 'Welcome to AuraCap',
      description:
        'AuraCap is a professional network packet analyzer designed to make packet analysis simple and accessible.',
      icon: '🔍',
    },
    {
      title: 'Start Capturing',
      description:
        'Click "Start Capture" to begin capturing network packets. You can filter by interface and apply BPF filters.',
      icon: '▶️',
    },
    {
      title: 'View Packet Details',
      description:
        'Click any packet in the list to see its details. The "Essentials" tab provides a beginner-friendly summary.',
      icon: '📋',
    },
    {
      title: 'Analyze Traffic',
      description:
        'Use the Hex View for raw bytes, Payload Inspector for decoded content, and Intelligence for risk analysis.',
      icon: '📊',
    },
    {
      title: 'Export & Share',
      description:
        'Export your captures as PCAP files to share with others or analyze later in other tools.',
      icon: '💾',
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
  <div
    class="w-full max-w-lg rounded-lg shadow-2xl overflow-hidden"
    style="background-color: var(--cursor-cream);"
  >
    <!-- Progress bar -->
    <div class="h-1" style="background-color: var(--surface-200);">
      <div
        class="h-full transition-all duration-300"
        style="width: {progress}%; background-color: var(--color-success);"
      ></div>
    </div>

    <div class="p-8">
      <!-- Step icon and number -->
      <div class="flex items-center justify-center mb-6">
        <div
          class="w-16 h-16 rounded-full flex items-center justify-center text-3xl"
          style="background-color: var(--surface-200);"
        >
          {steps[currentStep].icon}
        </div>
      </div>

      <!-- Step content -->
      <div class="text-center mb-8">
        <h2 class="text-xl font-bold mb-2" style="color: var(--cursor-dark);">
          {steps[currentStep].title}
        </h2>
        <p class="text-sm leading-relaxed" style="color: rgba(38, 37, 30, 0.7);">
          {steps[currentStep].description}
        </p>
      </div>

      <!-- Step indicators -->
      <div class="flex justify-center gap-2 mb-6">
        {#each steps as _step, i}
          <button
            class="w-2 h-2 rounded-full transition-all cursor-pointer"
            style="background-color: {i === currentStep
              ? 'var(--color-success)'
              : 'var(--surface-300)'}; border: none;"
            on:click={() => (currentStep = i)}
            aria-label="Go to step {i + 1}"
          ></button>
        {/each}
      </div>

      <!-- Actions -->
      <div class="flex items-center justify-between">
        <button
          class="px-4 py-2 text-sm font-medium transition-colors cursor-pointer"
          style="color: rgba(38, 37, 30, 0.55); background: transparent; border: none;"
          on:click={skip}
        >
          Skip Tour
        </button>

        <div class="flex gap-3">
          {#if currentStep > 0}
            <button
              class="px-4 py-2 text-sm font-medium rounded cursor-pointer transition-colors"
              style="background-color: var(--surface-200); color: var(--cursor-dark); border: 1px solid var(--border-primary);"
              on:click={previous}
            >
              Back
            </button>
          {/if}
          <button
            class="px-6 py-2 text-sm font-bold rounded cursor-pointer transition-colors"
            style="background-color: var(--color-success); color: white; border: none;"
            on:click={next}
          >
            {currentStep === steps.length - 1 ? 'Get Started' : 'Next'}
          </button>
        </div>
      </div>
    </div>
  </div>
</div>
