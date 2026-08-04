<script setup lang="ts">
import { Ref, ref, onMounted } from 'vue';
import Bran from './Bran.vue';
import Hodor from './Hodor.vue';
import { info_t, LEVEL } from './log';
import { now, useTimeoutFn, useTitle, useUrlSearchParams } from '@vueuse/core';

const searchParams = useUrlSearchParams();
const role = (searchParams.r as string)?.toLowerCase() == 'lord' ? Bran : Hodor;
const name = searchParams.n as (string | undefined);
const family = (typeof searchParams.f === 'string' && searchParams.f.length > 0) ? searchParams.f : 'Stark';
useTitle().value = `Hodor - ${family}`;
const expire = (typeof searchParams.e === 'string' && searchParams.e.length > 0) ? parseInt(searchParams.e) : undefined;
const msgShow = ref(false);
const msg = ref("");
const msgLevel = ref(LEVEL.INFO);
const timer = useTimeoutFn(updateProgress, 30);
let startTime = ref(0)
let stopTime = ref(0)
let progress: Ref<number | undefined> = ref(0)

// 邀请码检测 —— 有邀请码时显示右上角注册入口
const hasInvites = ref(false);
onMounted(async () => {
    try {
        const resp = await fetch('/gate/api/invite/exists');
        const data = await resp.json();
        hasInvites.value = !!data.has_invites;
    } catch (e) {
        console.error('check invite failed', e);
    }
});

const MAP_INFO_LEVEL = new Map([
  [LEVEL.INFO, "level-info"],
  [LEVEL.WARNING, "level-warning"],
  [LEVEL.ERROR, "level-error"],
  [LEVEL.SUCCESS, "level-success"],
]);

function pushMessage(info: info_t) {
  timer.stop();
  msg.value = info.msg;
  msgLevel.value = info.level;
  if (info.timeout > 0) {
    startTime.value = now();
    stopTime.value = startTime.value + info.timeout;
    timer.start();
  }
  else {
    progress.value = undefined;
  }
  msgShow.value = true;
}

function updateProgress() {
  let ts = now();
  if (ts > stopTime.value) {
    msgShow.value = false;
    return;
  }
  progress.value = stopTime.value - ts;
  timer.start();
}

function recall() {
  let url = new URL(window.location.href);
  if (url.searchParams.has('r')) {
    url.searchParams.delete('r');
  }
  else {
    url.searchParams.set('r', 'lord');
  }
  window.location.replace(url.href);
}

// 注册入口：点击切换到注册页（仅当存在邀请码时显示）
function goRegister() {
  let url = new URL(window.location.href);
  url.searchParams.set('r', 'lord');
  window.location.replace(url.href);
}

// 注册成功：等成功消息展示片刻后自动切回登录页，并带上真实 family 与 name
function onRegistered(...args: unknown[]) {
  const info = (args[0] ?? {}) as { family: string; name: string };
  window.setTimeout(() => {
    let url = new URL(window.location.href);
    url.searchParams.set('f', info.family);
    url.searchParams.set('n', info.name);
    url.searchParams.delete('r');
    window.location.replace(url.href);
  }, 2500);
}

</script>

<template>
  <!-- 右上角注册入口 —— 仅当存在邀请码时显示 -->
  <button
    v-if="hasInvites && role !== Bran"
    class="btn btn-xs btn-ghost fixed top-2 right-2 z-50"
    @click="goRegister"
  >注册</button>

  <div class="card w-80 shadow-2xl card-bordered">
    <div class="card-body w-full" @click.ctrl="recall">
      <component :is="role" @msg="pushMessage" @registered="onRegistered" :name="name" :family="family" :expire="expire">
      </component>
    </div>
  </div>
  <Transition name="popup">
    <div v-show="msgShow" class="fixed bottom-8 left-0 right-0 px-4 z-50 w-80 shadow-2xl mx-auto">
      <progress class="progress" :max="stopTime - startTime" :value="progress"></progress>
      <div :class="'hyphens-auto alert ' + MAP_INFO_LEVEL.get(msgLevel)">
        {{ msg }}
      </div>
    </div>
  </Transition>
</template>

<style scoped lang="postcss">
.is-active {
  @apply active font-bold;
}

.is-inactive {
  @apply text-secondary;
}

.level-info {
  @apply alert-info;
}

.level-warning {
  @apply alert-warning;
}

.level-error {
  @apply alert-error;
}

.level-success {
  @apply alert-success;
}

.popup-enter-from,
.popup-leave-to {
  opacity: 0;
}

.popup-enter-active,
.popup-leave-active {
  transition: all .5s ease;
}
</style>
