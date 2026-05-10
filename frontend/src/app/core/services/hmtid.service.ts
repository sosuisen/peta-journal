import { Injectable } from '@angular/core';
import { monotonicFactory } from 'hmtid';

@Injectable({ providedIn: 'root' })
export class HmtidService {
  private readonly generate = monotonicFactory(undefined, '-', true);

  nextId(): string {
    return this.generate();
  }
}
