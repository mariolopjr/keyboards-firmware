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
// IWYU pragma: begin_exports
#include QMK_KEYBOARD_H
#include "printf.h"
#include "version.h"
// IWYU pragma: end_exports

#define MACRO_TIMER 5

enum userspace_custom_keycodes {
    VRSN = SAFE_RANGE, // Prints QMK Firmware and board info
    SCL2,              // Sends Scroll Lock twice to toggle KM
    NEW_SAFE_RANGE
};
