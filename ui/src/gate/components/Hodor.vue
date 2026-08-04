<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { startAuthentication } from '@simplewebauthn/browser';
import { useLocalStorage } from '@vueuse/core'
import { useUrlSearchParams } from '@vueuse/core';
import { info_t, LEVEL } from './log';
import { h } from './utils';
import { base64URLStringToBuffer } from '@simplewebauthn/browser';

const searchParams = useUrlSearchParams();
const props = defineProps<{ name?: string, family: string, expire?: number }>();
const name = (props.name && props.name.length > 0) ? ref(props.name) : useLocalStorage(props.family, '');
const token = ref("");
const emit = defineEmits<{
    (event: 'msg', msg: info_t): void
}>();

const tokenGroup = computed(() => token.value.trim().split('.'));
const tokenValid = computed(() => tokenGroup.value.length == 3);
const DEV = import.meta.env.DEV;
const header = computed(() => tokenValid.value ? JSON.stringify(JSON.parse(base64URLDecode(tokenGroup.value[0])), null, 2) : null);
const payload = computed(() => tokenValid.value ? JSON.stringify(JSON.parse(base64URLDecode(tokenGroup.value[1])), null, 2) : null);

const url_prefix = "/gate/api/auth?";

function base64URLDecode(base64URLString: string) {
    return new TextDecoder().decode(base64URLStringToBuffer(base64URLString));
}

async function auth() {
    if (name.value.length == 0) {
        emit("msg", { level: LEVEL.WARNING, msg: "who are you?", timeout: 3000 });
        return;
    }
    let url = `${url_prefix}name=${name.value}&family=${props.family}`;
    emit("msg", { level: LEVEL.INFO, msg: 'fetch challenge', timeout: -1 });
    const resp = await h(fetch(url));
    if (resp.err || !resp.v?.ok) {
        emit("msg", { level: LEVEL.ERROR, msg: "fetch challenge failed", timeout: 3000 });
        return;
    }
    const respJson = await h(resp.v.json());
    if (respJson.err) {
        emit("msg", { level: LEVEL.ERROR, msg: "fetch challenge failed", timeout: 3000 });
        return;
    }
    emit("msg", { level: LEVEL.WARNING, msg: 'confirm?', timeout: respJson.v?.timeout ?? -1 });
    const asseResp = await h(startAuthentication(respJson.v));
    if (asseResp.err) {
        emit("msg", { level: LEVEL.ERROR, msg: `authenticate failed: ${asseResp.err.message}`, timeout: 3000 });
        return;
    }
    emit("msg", { level: LEVEL.INFO, msg: 'wait for verify', timeout: -1 });
    // ttl 由后端 meme.jwt_expire 配置决定，前端不传 expire 覆盖
    const verificationResp = await h(fetch(url, {
        method: "POST",
        headers: {
            'Content-Type': 'application/json'
        },
        body: JSON.stringify(asseResp.v),
    }));
    if (verificationResp.err || !verificationResp.v?.ok) {
        emit("msg", { level: LEVEL.ERROR, msg: `verification failed: ${verificationResp.err?.message}`, timeout: 3000 });
        return;
    }
    const result = await h(verificationResp.v.json());
    if (result.err) {
        emit("msg", { level: LEVEL.ERROR, msg: `verification failed: ${result.err.message}`, timeout: 3000 });
        return;
    }
    const pack = result.v as { token?: string; baggage?: object };
    token.value = pack.token ?? "";
    emit("msg", { level: LEVEL.SUCCESS, msg: `get token`, timeout: 5000 });

    if (searchParams.c?.length > 0) {
        let url = new URL(decodeURIComponent(searchParams.c as string));
        url.searchParams.set('token', token.value);
        if (pack.baggage) {
            url.searchParams.set('baggage', JSON.stringify(pack.baggage));
        }
        console.log(url);
        window.location.assign(url.href);
    }
}

onMounted(async () => {
    if (searchParams.t) {
        await auth();
    }
})
</script>

<template>
    <div class="grid grid-cols-1 my-2 gap-1">
        <input class="input input-bordered input-info input-lg text-center" placeholder="name" v-model="name" />
        <button class="btn btn-secondary btn-sm" @click="auth"></button>
        <div v-if="DEV" class="textarea textarea-bordered textarea-xs">
            <p class="text-xs break-all font-mono whitespace-pre">
                {{ header }}
            </p>
            <p class="text-xs break-all font-mono whitespace-pre">
                {{ payload }}
            </p>
        </div>
    </div>
</template>
