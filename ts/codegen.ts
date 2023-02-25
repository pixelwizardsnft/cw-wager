import codegen from '@cosmwasm/ts-codegen'

codegen({
  contracts: [
    {
      name: 'Wager',
      dir: '../schema',
    },
  ],
  outPath: './types/',

  // options are completely optional ;)
  options: {
    bundle: {
      enabled: false,
    },
    types: {
      enabled: true,
    },
    client: {
      enabled: true,
    },
    reactQuery: {
      enabled: false,
    },
    recoil: {
      enabled: false,
    },
    messageComposer: {
      enabled: true,
    },
  },
}).then(() => {
  console.log('✨ all done!')
})
