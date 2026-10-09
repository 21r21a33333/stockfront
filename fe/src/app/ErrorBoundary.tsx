import { Component, type ReactNode } from "react";

/** Keeps an unexpected render error from blanking the page while a run may be in flight. */
export class ErrorBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    if (!this.state.failed) return this.props.children;
    return (
      <main
        style={{
          minHeight: "calc(100vh / var(--ui-scale))",
          display: "grid",
          placeItems: "center",
          padding: 24,
          textAlign: "center",
        }}
      >
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 14,
            maxWidth: 440,
          }}
        >
          <h1 className="disp" style={{ margin: 0, fontSize: 36 }}>
            Something went wrong.
          </h1>
          <p style={{ margin: 0, color: "var(--ink-3)", lineHeight: 1.5 }}>
            Nothing was sent from this page. Any transaction you already signed
            is still tracked in your wallet and on BNB Chain.
          </p>
          <button
            type="button"
            onClick={() => window.location.reload()}
            style={{
              alignSelf: "center",
              height: 44,
              padding: "0 18px",
              borderRadius: 12,
              border: 0,
              background: "var(--ink)",
              color: "#fff",
            }}
          >
            Reload
          </button>
        </div>
      </main>
    );
  }
}
