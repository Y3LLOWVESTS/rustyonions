import React from 'react'

export function PlaneStatusTable() {
  return (
    <table>
      <thead>
        <tr>
          <th>Plane</th>
          <th>Health</th>
          <th>Ready</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>gateway</td>
          <td>healthy</td>
          <td>true</td>
        </tr>
      </tbody>
    </table>
  )
}
