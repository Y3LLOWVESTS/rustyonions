/**
 * Ron client
 *
 * High-level client for calling /app/* on a RON-CORE node.
 * This is a scaffold; methods will be fully implemented later.
 */

import type { AppRequest, AppResponse, RonOptions } from './types';

export class Ron {
  private readonly options: RonOptions;

  constructor(options: RonOptions) {
    this.options = options;
  }

  async request<T = unknown>(_req: AppRequest): Promise<AppResponse<T>> {
    // TODO: implement transport + error mapping
    throw new Error('Ron.request not implemented yet (scaffold)');
  }

  async get<T = unknown>(
    path: string,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'GET', path, query });
  }

  async post<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'POST', path, body, query });
  }

  async put<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'PUT', path, body, query });
  }

  async patch<T = unknown>(
    path: string,
    body?: unknown,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'PATCH', path, body, query });
  }

  async delete<T = unknown>(
    path: string,
    query?: Record<string, string>,
  ): Promise<AppResponse<T>> {
    return this.request<T>({ method: 'DELETE', path, query });
  }
}
