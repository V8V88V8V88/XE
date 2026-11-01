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

    <section class="xe-showcase xe-reveal" aria-labelledby="xe-showcase-heading">
      <h2 id="xe-showcase-heading" class="xe-sr-only">What XE code looks like</h2>
      <div class="xe-window">
        <div class="xe-pane">
          <div class="xe-pane-bar"><span class="xe-pane-name">fib.xe</span></div>
<pre class="xe-code"><code><span class="k">fun</span> <span class="f">fib</span>(n):
    <span class="k">if</span> n &lt;= <span class="n">1</span>:
        <span class="k">return</span> n
    <span class="k">return</span> <span class="f">fib</span>(n - <span class="n">1</span>) + <span class="f">fib</span>(n - <span class="n">2</span>)

<span class="k">for</span> n <span class="k">in</span> [<span class="n">5</span>, <span class="n">10</span>, <span class="n">20</span>]:
    <span class="f">print</span>(<span class="s">"fib"</span>, n, <span class="s">"="</span>, <span class="f">fib</span>(n))</code></pre>
        </div>
        <div class="xe-pane xe-pane-term">
          <div class="xe-pane-bar"><span class="xe-pane-name">Terminal</span></div>
<pre class="xe-code"><code><span class="p">$</span> xe run fib.xe
<span class="o">fib 5 = 5
fib 10 = 55
fib 20 = 6765</span>

<span class="c"># or build a standalone binary</span>
<span class="p">$</span> xe compile fib.xe -o fib
<span class="p">$</span> ./fib</code></pre>
        </div>
      </div>
    </section>

    <section id="install" class="xe-install-bar xe-reveal" aria-labelledby="xe-install-heading">
      <div class="xe-install-content">
        <h2 id="xe-install-heading" class="xe-install-title">
          <span class="xe-install-kicker">Install</span>
          <span class="xe-install-headline">Get the latest release in one command</span>
        </h2>
        <div class="xe-command-box" role="group" aria-label="Install command">
          <code id="xe-install-command"><span class="xe-prompt" aria-hidden="true">$ </span>curl -fsSL https://xe-lang.vercel.app/install.sh | bash</code>
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
          Works on Linux, macOS, and FreeBSD. On Windows, grab the zip from
          <a href="https://github.com/V8V88V8V88/XE/releases" target="_blank" rel="noopener noreferrer">GitHub Releases</a> or use <code>cargo install</code>.
        </p>
      </div>
    </section>

    <section class="xe-features xe-reveal" aria-labelledby="xe-features-heading">
      <h2 id="xe-features-heading" class="xe-sr-only">Why XE</h2>
      <div class="xe-feature-grid">
        <div v-for="item in features" :key="item.title" class="xe-feature">
          <h3>{{ item.title }}</h3>
          <p>{{ item.body }}</p>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup>
import { ref, onMounted, onUnmounted } from 'vue'

const copied = ref(false)
const showScrollDown = ref(true)

let installInView = false
let installObserver = null

// Only show the arrow when the install box is below the fold
const handleScroll = () => {
  showScrollDown.value = window.scrollY < 100 && !installInView
}

onMounted(() => {
  window.addEventListener('scroll', handleScroll, { passive: true })
  const install = document.getElementById('install')
  if (install && 'IntersectionObserver' in window) {
    installObserver = new IntersectionObserver(([entry]) => {
      installInView = entry.isIntersecting
      handleScroll()
    }, { threshold: 0 })
    installObserver.observe(install)
  }
})

onUnmounted(() => {
  window.removeEventListener('scroll', handleScroll)
  installObserver?.disconnect()
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
    const textArea = document.createElement('textarea')
    textArea.value = command
    document.body.appendChild(textArea)
    textArea.select()
    try {
      document.execCommand('copy')
      copied.value = true
    } catch (e) {}
    document.body.removeChild(textArea)
  }

  if (copied.value) {
    setTimeout(() => {
      copied.value = false
    }, 2000)
  }
}

const features = [
  {
    title: 'Readable syntax',
    body: 'Indentation-based blocks, no semicolons or braces. Code that reads like pseudocode.'
  },
  {
    title: 'Rust under the hood',
    body: 'XE compiles to Rust and builds with rustc, so programs get its optimizer and safety checks.'
  },
  {
    title: 'One native binary',
    body: 'xe compile produces a standalone executable. No VM or runtime to ship alongside it.'
  }
]
</script>
