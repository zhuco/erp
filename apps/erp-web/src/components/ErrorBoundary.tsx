import { Button, Result } from 'antd';
import type { ReactNode } from 'react';
import { Component } from 'react';

type ErrorBoundaryProps = {
  children: ReactNode;
};

type ErrorBoundaryState = {
  error?: Error;
};

export class ErrorBoundary extends Component<ErrorBoundaryProps, ErrorBoundaryState> {
  state: ErrorBoundaryState = {
    error: undefined
  };

  static getDerivedStateFromError(error: Error): ErrorBoundaryState {
    return { error };
  }

  private handleReset = () => {
    this.setState({ error: undefined });
  };

  render() {
    const { error } = this.state;
    if (error) {
      return (
        <Result
          status="error"
          title="页面出现错误"
          subTitle={error.message}
          extra={<Button onClick={this.handleReset}>重试</Button>}
        />
      );
    }
    return this.props.children;
  }
}
