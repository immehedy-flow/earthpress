import { Component, type ErrorInfo, type ReactNode } from 'react'
import { AlertTriangle, RefreshCw } from 'lucide-react'

interface Props {
  children: ReactNode
}

interface State {
  hasError: boolean
  error: Error | null
}

export class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props)
    this.state = { hasError: false, error: null }
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error }
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('EarthPress error:', error, info.componentStack)
  }

  handleReset = () => {
    this.setState({ hasError: false, error: null })
  }

  render() {
    if (this.state.hasError) {
      return (
        <div style={{
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          justifyContent: 'center',
          minHeight: '100%',
          padding: '24px',
          textAlign: 'center',
          color: 'var(--tx)',
          background: 'var(--bg)',
          gap: '16px'
        }}>
          <AlertTriangle size={48} style={{ color: 'var(--danger)' }} />
          <h2 style={{ margin: 0, fontFamily: 'var(--font-display)', fontSize: '20px' }}>
            Something went wrong
          </h2>
          <p style={{ color: 'var(--tx3)', maxWidth: '400px', margin: 0 }}>
            {this.state.error?.message || 'An unexpected error occurred.'}
          </p>
          <button
            onClick={this.handleReset}
            className="btn btn--primary"
            style={{ borderRadius: '12px', height: '40px', padding: '0 16px' }}
          >
            <RefreshCw size={16} />
            Restart app
          </button>
        </div>
      )
    }

    return this.props.children
  }
}