<script setup lang="ts">
import { startRegistration } from '@simplewebauthn/browser';
import { useLocalStorage } from '@vueuse/core'
import { ref } from 'vue';
import { LEVEL, info_t } from './log';
import { Err, h } from './utils';

const props = defineProps<{ name?: string, family: string, expire?: any }>();
const name = (props.name && props.name.length > 0) ? ref(props.name) : useLocalStorage(props.family, '');
const code = ref("");
const baggage = ref("");
const emit = defineEmits<{
    (event: 'msg', msg: info_t): void
    (event: 'registered', info: { family: string; name: string }): void
}>();

const url_prefix = "/gate/api/register?";

const ERROR_MAP: Record<string, string> = {
    invalid_code: '邀请码无效，请检查后重试',
    invite_expired: '邀请码已过期，请联系管理员重新获取',
    already_registered: '该名字已注册，请直接登录',
    no_active_challenge: '注册会话已失效，请重新开始',
};

async function register() {
    if (name.value.length == 0) {
        emit("msg", { level: LEVEL.WARNING, msg: "who are you?", timeout: 3000 });
        return;
    }
    let url = `${url_prefix}name=${name.value}&family=${props.family}`;
    emit("msg", { level: LEVEL.INFO, msg: "fetch challenge", timeout: -1 });
    const resp = await h(fetch(url, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify({
            code: code.value,
        }),
    }));
    if (resp.err || !resp.v?.ok) {
        let detail = "fetch challenge failed";
        if (resp.v) {
            try {
                const errData = await resp.v.json();
                if (errData?.error) detail = ERROR_MAP[errData.error] ?? errData.error;
            } catch (_) { /* ignore */ }
        }
        emit("msg", { level: LEVEL.ERROR, msg: detail, timeout: 5000 });
        return;
    }
    const respJson = await h(resp.v.json());
    if (respJson.err) {
        emit("msg", { level: LEVEL.ERROR, msg: "fetch challenge failed", timeout: 3000 });
        return;
    }
    emit("msg", { level: LEVEL.WARNING, msg: "confirm?", timeout: respJson.v?.timeout ?? -1 });
    let attResp: Err<any> = await h(startRegistration(respJson.v));
    if (attResp.err) {
        emit("msg", { level: LEVEL.ERROR, msg: `register failed: ${attResp.err.message}`, timeout: 3000 });
        return;
    }
    emit("msg", { level: LEVEL.INFO, msg: "wait for verify", timeout: -1 });
    try {
        if (baggage.value.length > 0) {
            attResp.v!.baggage = JSON.parse(baggage.value);
        }
    }
    catch (err) {
        emit("msg", { level: LEVEL.ERROR, msg: "parse baggage to JSON failed", timeout: 3000 });
        return;
    }
    let verificationResp = await h(fetch(url, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
        },
        body: JSON.stringify(attResp.v),
    }));
    if (verificationResp.err || !verificationResp.v?.ok) {
        emit("msg", { level: LEVEL.ERROR, msg: `register failed`, timeout: 3000 });
        return;
    }
    const result = await h(verificationResp.v.json());
    if (result.err) {
        emit("msg", { level: LEVEL.ERROR, msg: `register failed`, timeout: 3000 });
        return;
    }
    emit("msg", { level: LEVEL.SUCCESS, msg: `register succeed`, timeout: -1 });
    // 注册成功 → 通知 Winterfell 稍后切回登录页，并带上真实 family
    const registeredFamily = (result.v as { family?: string } | undefined)?.family || props.family;
    emit('registered', { family: registeredFamily, name: name.value });
}
</script>

<template>
    <div class="grid grid-cols-1 my-2 gap-1">
        <input class="input input-bordered input-info input-lg" placeholder="name" v-model="name" />
        <input class="input input-bordered input-info" placeholder="code" type="password" v-model="code" />
        <textarea class="textarea textarea-bordered" placeholder="baggage" v-model="baggage"></textarea>
        <button class="btn btn-warning btn-sm" @click="register"></button>
    </div>
</template>
