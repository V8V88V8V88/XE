<template>
  <div class="xe-home">
    <section id="install" class="xe-install-bar xe-reveal">
      <p class="xe-install-kicker">Install</p>
      <p class="xe-install-title">The cleanest install path today is through Cargo.</p>
      <div class="xe-command">
        <code>cargo install --git https://github.com/V8V88V8V88/XE --bin xe</code>
      </div>
      <p class="xe-install-note">
        Requires the Rust toolchain. If you want to study or modify the compiler, use the
        build-from-source guide instead.
      </p>
    </section>

    <section class="xe-grid">
      <article class="xe-feature xe-panel xe-reveal xe-delay-1">
        <h3>Readable on the surface</h3>
        <p>
          XE uses indentation-based syntax, straightforward control flow, and a small standard
          vocabulary so the language stays easy to follow in demos and code reviews.
        </p>
      </article>
      <article class="xe-feature xe-panel xe-reveal xe-delay-2">
        <h3>Rust underneath</h3>
        <p>
          Programs are lowered into Rust source and then compiled with <code>rustc</code>. You get
          a concrete compiler pipeline instead of a black-box runtime.
        </p>
      </article>
      <article class="xe-feature xe-panel xe-reveal xe-delay-3">
        <h3>Small enough to understand</h3>
        <p>
          Lexer, parser, semantic checks, and code generation all fit into a compact codebase. That
          makes XE useful as both a language and a compiler project.
        </p>
      </article>
    </section>

    <section class="xe-dual">
      <article class="xe-panel xe-copy xe-reveal xe-delay-1">
        <p class="xe-section-kicker">Why it works</p>
        <h2 class="xe-section-title">A minimal language with a real compilation story.</h2>
        <p>
          XE is not trying to be everything. It focuses on readable syntax, explicit control flow,
          and a compiler pipeline you can explain in a classroom, a report, or a live demo without
          hand-waving.
        </p>
        <p>
          The current implementation supports functions, recursion, lists, indexing,
          <code>if / elif / else</code>, <code>repeat</code>, <code>while</code>, <code>for</code>,
          and loop control with clear runtime failures for invalid operations.
        </p>
        <div class="xe-inline-links">
          <a href="/guide/language-basics">Read the language guide</a>
          <a href="/reference/status">See implementation status</a>
          <a href="/guide/examples">Run the examples</a>
        </div>
      </article>

      <div class="xe-code-stack">
        <article class="xe-code-block xe-reveal xe-delay-2">
          <div class="xe-code-label"><span>XE Source</span><span>hello.xe</span></div>
          <pre><code>function greet(name):
    print("Hello, " + name)

for person in ["World", "Rust"]:
    greet(person)</code></pre>
        </article>

        <article class="xe-code-block xe-reveal xe-delay-3">
          <div class="xe-code-label"><span>Run</span><span>native output</span></div>
          <pre><code>xe run hello.xe

Hello, World
Hello, Rust</code></pre>
        </article>
      </div>
    </section>

    <section class="xe-pipeline xe-reveal">
      <article class="xe-stage">
        <span>01</span>
        <strong>Write XE</strong>
        <p>Use compact, indentation-based syntax.</p>
      </article>
      <article class="xe-stage">
        <span>02</span>
        <strong>Lex + Parse</strong>
        <p>Turn source text into tokens and an AST.</p>
      </article>
      <article class="xe-stage">
        <span>03</span>
        <strong>Validate</strong>
        <p>Check names, arity, loop control, and structure.</p>
      </article>
      <article class="xe-stage">
        <span>04</span>
        <strong>Generate Rust</strong>
        <p>Emit explicit Rust code for the XE program.</p>
      </article>
      <article class="xe-stage">
        <span>05</span>
        <strong>Build Native</strong>
        <p>Let <code>rustc</code> produce the final executable.</p>
      </article>
    </section>

    <section class="xe-status">
      <article class="xe-status-card xe-reveal xe-delay-1">
        <p class="xe-section-kicker">What you can use now</p>
        <h3>Current language surface</h3>
        <ul class="xe-status-list">
          <li>numbers, text, booleans, and lists</li>
          <li>functions, recursion, and returns</li>
          <li><code>if / elif / else</code>, <code>repeat</code>, <code>while</code>, and <code>for</code></li>
          <li><code>break</code>, <code>continue</code>, indexing, and built-ins</li>
          <li>compiler diagnostics with source lines and carets</li>
        </ul>
      </article>

      <article class="xe-status-card xe-reveal xe-delay-2">
        <p class="xe-section-kicker">Project shape</p>
        <h3>Built to present clearly</h3>
        <p>
          XE already has a working compiler pipeline, integration tests, CI, examples, and
          reference docs. It is still pre-alpha, but it is now in a shape that reads as a coherent
          language project instead of a loose prototype.
        </p>
        <div class="xe-inline-links">
          <a href="/guide/getting-started">Install and run XE</a>
          <a href="/reference/cli">Open the CLI reference</a>
        </div>
      </article>
    </section>
  </div>
</template>
