/* Copyright 2023 mariolopjr
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */
#pragma once

// how long CTL_ESC has to be held before it resolves to Ctrl
#undef TAPPING_TERM
#define TAPPING_TERM 150

// pressing any other key while CTL_ESC is held resolves it to Ctrl right away,
// instead of waiting out TAPPING_TERM
#define HOLD_ON_OTHER_KEY_PRESS

// while typing, CTL_ESC pressed within this long of the previous key is always a
// tap
#define FLOW_TAP_TERM 100

#undef DEBOUNCE
#define DEBOUNCE 5
