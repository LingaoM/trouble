/* SPDX-License-Identifier: MIT OR Apache-2.0 */

#include <stdint.h>
#include <string.h>

#include "bs_pc_base.h"

static pb_dev_state_t g_device;
static uint64_t g_now_us;
static int g_connected;

int trouble_bsim_init(const char *sim_id, const char *phy_id,
                      unsigned int device_id)
{
  int ret;

  if (sim_id == NULL || phy_id == NULL || g_connected)
    {
      return -1;
    }

  memset(&g_device, 0, sizeof(g_device));
  ret = pb_dev_init_com(&g_device, device_id, sim_id, phy_id);
  if (ret != 0)
    {
      return ret;
    }

  g_now_us = 0;
  g_connected = 1;
  return 0;
}

int trouble_bsim_step(uint64_t target_us, uint64_t *actual_us)
{
  pb_wait_t wait;
  int ret;

  if (!g_connected || actual_us == NULL)
    {
      return -1;
    }

  if (target_us > g_now_us)
    {
      wait.end = (bs_time_t)target_us;
      ret = pb_dev_request_wait_block(&g_device, &wait);
      if (ret < 0)
        {
          return ret;
        }

      g_now_us = (uint64_t)wait.end;
    }

  *actual_us = g_now_us;
  return 0;
}

void trouble_bsim_disconnect(void)
{
  if (g_connected)
    {
      pb_dev_disconnect(&g_device);
      g_connected = 0;
    }
}
