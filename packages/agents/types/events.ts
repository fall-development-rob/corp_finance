// Domain events across all bounded contexts
// Used for event-driven communication between contexts

export type DomainEventType =
  // BC1: Analysis Orchestration
  | 'AnalysisRequested'
  | 'PlanCreated'
  | 'AnalystAssigned'
  | 'ResultAggregated'
  | 'AnalysisEscalated'
  // BC2: Specialist Analysts
  | 'ToolCalled'
  | 'ToolSucceeded'
  | 'ToolFailed'
  | 'AnalysisCompleted'
  | 'InsightGenerated'
  // BC3: Financial Memory
  | 'MemoryStored'
  | 'MemoryRetrieved'
  | 'PatternDiscovered'
  // BC4: Learning & Adaptation
  | 'PatternLearned'
  | 'StrategyAdapted'
  | 'FeedbackReceived';

export interface DomainEvent<T = unknown> {
  eventId: string;
  type: DomainEventType;
  timestamp: Date;
  sourceContext: string;   // bounded context name
  payload: T;
}

export interface EventBus {
  emit(event: DomainEvent): void;
  on(type: DomainEventType, handler: (event: DomainEvent) => void): void;
  off(type: DomainEventType, handler: (event: DomainEvent) => void): void;
}

// Simple in-process event bus implementation
export class SimpleEventBus implements EventBus {
  private handlers = new Map<DomainEventType, Set<(event: DomainEvent) => void>>();

  emit(event: DomainEvent): void {
    const typeHandlers = this.handlers.get(event.type);
    if (typeHandlers) {
      for (const handler of typeHandlers) {
        handler(event);
      }
    }
  }

  on(type: DomainEventType, handler: (event: DomainEvent) => void): void {
    if (!this.handlers.has(type)) {
      this.handlers.set(type, new Set());
    }
    this.handlers.get(type)!.add(handler);
  }

  off(type: DomainEventType, handler: (event: DomainEvent) => void): void {
    this.handlers.get(type)?.delete(handler);
  }
}
