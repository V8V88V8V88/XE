<template>
  <div class="xe-home">
    <button
      v-if="showScrollDown"
      type="button"
      class="xe-scroll-down"
      @click="scrollToInstall"
      aria-label="Scroll to install section"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true">
        <path d="M7 13l5 5 5-5M7 6l5 5 5-5"/>
      </svg>
    </button>

    <section id="install" class="xe-install-bar xe-reveal" aria-labelledby="xe-install-heading">
      <div class="xe-install-content">
        <h2 id="xe-install-heading" class="xe-install-title">
          <span class="xe-install-kicker">Install</span>
          <span class="xe-install-headline">Install the latest XE release binary in one command.</span>
        </h2>
        <div class="xe-command-box" role="group" aria-label="Install command">
          <code id="xe-install-command">curl -fsSL https://xe-lang.vercel.app/install.sh | bash</code>
          <button
            type="button"
            class="xe-copy-trigger"
            @click="copyCommand"
            :aria-label="copied ? 'Copied to clipboard' : 'Copy install command'"
          >
            <svg v-if="!copied" xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"></rect><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"></path></svg>
            <svg v-else xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="var(--vp-c-brand-1)" stroke-width="3" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><polyline points="20 6 9 17 4 12"></polyline></svg>
          </button>
        </div>
        <p class="xe-install-note">
          macOS and Linux are supported by the installer today. Windows users can download the zip
          from <a href="https://github.com/V8V88V8V88/XE" target="_blank" rel="noopener noreferrer">GitHub</a>.
        </p>
      </div>
    </section>

    <section id="faq" class="xe-faq xe-reveal" aria-labelledby="xe-faq-heading">
      <h2 id="xe-faq-heading" class="xe-section-title">FAQ</h2>
      <div class="xe-faq-list">
        <div
          v-for="(item, index) in faqItems"
          :key="index"
          class="xe-faq-item"
          :class="{ 'xe-faq-active': activeFaq === index }"
        >
          <button
            type="button"
            class="xe-faq-question"
            :id="`${faqId}-q-${index}`"
            :aria-expanded="activeFaq === index"
            :aria-controls="`${faqId}-a-${index}`"
            @click="activeFaq = activeFaq === index ? null : index"
          >
            <span>{{ item.question }}</span>
            <span class="xe-faq-icon" aria-hidden="true">{{ activeFaq === index ? '−' : '+' }}</span>
          </button>
          <div
            :id="`${faqId}-a-${index}`"
            class="xe-faq-answer"
            role="region"
            :aria-labelledby="`${faqId}-q-${index}`"
            :aria-hidden="activeFaq !== index"
          >
            <p>{{ item.answer }}</p>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted, useId } from 'vue'

const faqId = useId()
const activeFaq = ref(null)
const copied = ref(false)
const showScrollDown = ref(true)

const handleScroll = () => {
  showScrollDown.value = window.scrollY < 100
}

const handleKeydown = (e) => {
  if (e.key === 'Escape' && activeFaq.value !== null) {
    activeFaq.value = null
  }
}

onMounted(() => {
  window.addEventListener('scroll', handleScroll, { passive: true })
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('scroll', handleScroll)
  window.removeEventListener('keydown', handleKeydown)
})

const scrollToInstall = () => {
  const el = document.getElementById('install')
  if (el) {
    el.scrollIntoView({ behavior: 'smooth', block: 'center' })
  }
}

const copyCommand = async () => {
  const command = 'curl -fsSL https://xe-lang.vercel.app/install.sh | bash'
  try {
    await navigator.clipboard.writeText(command)
    copied.value = true
  } catch (err) {
    // Fallback for older browsers
    const textArea = document.createElement("textarea");
    textArea.value = command;
    document.body.appendChild(textArea);
    textArea.select();
    try {
      document.execCommand('copy');
      copied.value = true;
    } catch (e) {}
    document.body.removeChild(textArea);
  }
  
  if (copied.value) {
    setTimeout(() => {
      copied.value = false
    }, 2000)
  }
}

const faqItems = [
  {
    question: 'What exactly is XE?',
    answer: 'XE is a small programming language designed for learning and rapid prototyping. It features an indentation-based syntax similar to Python but compiles directly into native Rust source code, which is then built into a standalone executable.'
  },
  {
    question: 'Why compile to Rust instead of using an interpreter?',
    answer: 'Compiling to Rust allows XE to leverage one of the world\'s most powerful compiler backends (LLVM through rustc). This provides massive performance benefits, exhaustive static analysis, and tiny, standalone binaries without requiring a heavy virtual machine or runtime environment.'
  },
  {
    question: 'Do I need Rust installed to use XE?',
    answer: 'Yes, XE requires the Rust toolchain (specifically rustc) to build your programs into binaries. However, the XE compiler itself handles the heavy lifting of code generation and binary invocation automatically.'
  },
  {
    question: 'Is XE memory safe?',
    answer: 'Because XE code is transpiled into high-level Rust code that follows Rust\'s ownership and safety rules, the resulting binaries benefit from the memory safety guarantees of the Rust language backend.'
  },
  {
    question: 'How does the module system work?',
    answer: 'XE uses a naming-mangled module system. When you import a module, the compiler assigns it a unique ID and renames its internal symbols to prevent conflicts, allowing for complex multi-file projects without namespace pollution.'
  },
  {
    question: 'Is this ready for production use?',
    answer: 'XE is currently in the "Pre-Alpha" stage. It is an excellent platform for learning about compiler design and language theory, but it is not yet intended for mission-critical production systems.'
  }
]
</script>

<style scoped>
/* Scoped styles can be added here if needed, but we are using custom.css */
</style>
