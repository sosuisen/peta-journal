import { Component, Suspense, type ReactNode } from 'react';

interface Props {
  /** Drawn while a part waits for the core. */
  readonly fallback?: ReactNode;
  readonly children: ReactNode;
}

/**
 * A part of the frame of the app: the header, the sidebar, the page or the footer. It waits
 * for the core by itself, and an error in it leaves the other parts as they are.
 */
export default class Frame extends Component<Props, { failed: boolean }> {
  override state = { failed: false };

  static getDerivedStateFromError(): { failed: boolean } {
    return { failed: true };
  }

  override render(): ReactNode {
    if (this.state.failed) {
      return (
        <p className="frame-error" role="alert">
          This part could not be drawn. Reload the page.
        </p>
      );
    }
    return (
      <Suspense fallback={this.props.fallback}>{this.props.children}</Suspense>
    );
  }
}
