# Changelog

## [0.2.5](https://github.com/mhatzl/mantra/compare/v0.2.4...v0.2.5) (2023-09-12)


### Bug Fixes

* escape newlines in release report ([60e4aff](https://github.com/mhatzl/mantra/commit/60e4affdfa7296b35ba5ccde4877089f8448be6b))

## [0.2.4](https://github.com/mhatzl/mantra/compare/v0.2.3...v0.2.4) (2023-09-12)


### Bug Fixes

* checkout project repo for report upload ([6a55c3d](https://github.com/mhatzl/mantra/commit/6a55c3d1674f06f0ed0c131e82f875f23f8a7e6d))

## [0.2.3](https://github.com/mhatzl/mantra/compare/v0.2.2...v0.2.3) (2023-09-12)


### Bug Fixes

* add GITHUB_TOKEN for release report upload ([1e59207](https://github.com/mhatzl/mantra/commit/1e59207112a2faefe362a519175dde5a5c36731c))
* remove dot from report extension ([43f71a8](https://github.com/mhatzl/mantra/commit/43f71a88da8e38b664fb70e0d53afa5e2a20a858))

## [0.2.2](https://github.com/mhatzl/mantra/compare/v0.2.1...v0.2.2) (2023-09-12)


### Bug Fixes

* split release report into multiple jobs ([0dbadf0](https://github.com/mhatzl/mantra/commit/0dbadf065e1a1a9ccbdec02ae7cf97fb50c49c34))

## [0.2.1](https://github.com/mhatzl/mantra/compare/v0.2.0...v0.2.1) (2023-09-12)


### Bug Fixes

* add mantra container to release action ([24fc391](https://github.com/mhatzl/mantra/commit/24fc39143186dc4d9d304b6344267127fb512799))

## [0.2.0](https://github.com/mhatzl/mantra/compare/v0.1.0...v0.2.0) (2023-09-12)


### Features

* add action to build&push docker image ([da8164c](https://github.com/mhatzl/mantra/commit/da8164c2119440f6b0a3a36a8faefeb265f89e6b))
* add action to test docker image ([119cbaf](https://github.com/mhatzl/mantra/commit/119cbaf74f9c435a4033e5fde58ab5225dfc03c5))
* add branch links to new ref entries ([09f5804](https://github.com/mhatzl/mantra/commit/09f5804834f56ed7869a51a60111b7cc305332d4))
* add check command ([#14](https://github.com/mhatzl/mantra/issues/14)) ([2eef2e4](https://github.com/mhatzl/mantra/commit/2eef2e47dc3c0a5d5348eb7708c45878c8a5fd1f))
* add check overview as pr comment ([edb2266](https://github.com/mhatzl/mantra/commit/edb2266a7fb463cd15932955df2aacf0ee2835c3))
* add check-pr workfow ([e3888e4](https://github.com/mhatzl/mantra/commit/e3888e471799ad59ce152725b810113ac58af158))
* add checklist for manual requirements ([8cba5d9](https://github.com/mhatzl/mantra/commit/8cba5d958d8d8d28018597fa471a9f0ae077e85e))
* add ci for mantra crate ([3012a52](https://github.com/mhatzl/mantra/commit/3012a5244f6c3f22c81ab91464beb5335221a821))
* add ignore file filter for referencing ([#22](https://github.com/mhatzl/mantra/issues/22)) ([164c58c](https://github.com/mhatzl/mantra/commit/164c58ced47aead89cb0b5fc1a2ebea56fe59034))
* add manual flag to ref list entries ([58c8531](https://github.com/mhatzl/mantra/commit/58c853171cdfd55ca19098f2fdfd9b6421a30621)), closes [#4](https://github.com/mhatzl/mantra/issues/4)
* add multi-repo support ([#21](https://github.com/mhatzl/mantra/issues/21)) ([74bcce0](https://github.com/mhatzl/mantra/commit/74bcce00bbb3b0289f2b12f0d876e61c3dd983de))
* add release report ([353384d](https://github.com/mhatzl/mantra/commit/353384ddbe92350349ad9981fabb4fdda9596b8b)), closes [#3](https://github.com/mhatzl/mantra/issues/3)
* add status cmd ([#18](https://github.com/mhatzl/mantra/issues/18)) ([abd870f](https://github.com/mhatzl/mantra/commit/abd870f282dbcfd8bb345c6c0fdc9aeedb947a05))
* add sync job to mantra workflow ([eb75841](https://github.com/mhatzl/mantra/commit/eb758412e1a8dfd339b8275df195dcea4d13b476))
* attach release report to release action ([0e0d574](https://github.com/mhatzl/mantra/commit/0e0d5748a80faee3642c2f60827709bbe3fa48b1))
* create basic docker image for mantra ([b01e6fd](https://github.com/mhatzl/mantra/commit/b01e6fde138ba4b7ef60bd343d7d9c219fee1da4))
* handle deprecated flag per branch ([252a032](https://github.com/mhatzl/mantra/commit/252a032e096fc16b56ab8e3cafb8503b8769f599)), closes [#7](https://github.com/mhatzl/mantra/issues/7)
* push link changes to main automatically ([ca01d2b](https://github.com/mhatzl/mantra/commit/ca01d2b4482e38a9ba04f31460495c8aa4448c7a))
* trigger mantra workflow on push to main ([f94abdb](https://github.com/mhatzl/mantra/commit/f94abdb042cf98f9ac500a3c7fd3eb3d8235fa76))
* update sidebar mantra info in wiki ([#23](https://github.com/mhatzl/mantra/issues/23)) ([2c62734](https://github.com/mhatzl/mantra/commit/2c6273407641f5cf415ab34d4d26c2c0a37d763d))


### Bug Fixes

* adapt dockerfile to run mantra from actions ([41dc590](https://github.com/mhatzl/mantra/commit/41dc5901beac992893ed2f8243bd489b01be131d))
* add git to dockerfile and allow git push ([d4c4188](https://github.com/mhatzl/mantra/commit/d4c4188dc7f5459e57243c70c390e544d5cd595c))
* add non-zero exit code on error ([2274ef8](https://github.com/mhatzl/mantra/commit/2274ef8c1dd7e22c030fab667c638a7811203add))
* add pull-request write permission ([3c47161](https://github.com/mhatzl/mantra/commit/3c47161c44673cb9b53b1bacee1f5c53b4003ddf))
* add ref for ref_req.ignore ([3689b0f](https://github.com/mhatzl/mantra/commit/3689b0f8f6b522cb5a675f5132b2dd92965b1e7e))
* add references for branch link req ([a472d0b](https://github.com/mhatzl/mantra/commit/a472d0b021b3f9f9e21270664bd21a43bcadcd54))
* convert check output for summary ([9f7cad9](https://github.com/mhatzl/mantra/commit/9f7cad93bdd00a5a9d005721358be3552f1fd953))
* enforce non-whitespace for req IDs ([d6cc9a2](https://github.com/mhatzl/mantra/commit/d6cc9a2392347827fba8de4eef4d3972ba426813))
* escape escaped newline ([cb301b3](https://github.com/mhatzl/mantra/commit/cb301b3499039cb04b5c588455493739ecf6dfab))
* escape parentheses in check summary ([7b3850b](https://github.com/mhatzl/mantra/commit/7b3850b3909bc533a9ac6dd67470da2b708a9c84))
* escape quote chars ([850cf31](https://github.com/mhatzl/mantra/commit/850cf3144eade24865fa34ec6b10cab84bb87209))
* escape quotes in replace pattern ([2a390f5](https://github.com/mhatzl/mantra/commit/2a390f5914c060cfc1dba01e762a9ee3ce4b18a9))
* escape quotes in summary ([bce3f62](https://github.com/mhatzl/mantra/commit/bce3f62b8e02b352a818ce2441fa4391af409193))
* forward stdout and stderr to overview ([f17dab0](https://github.com/mhatzl/mantra/commit/f17dab0d66683f968a487811d52e44c93af997da))
* give full write permissions for testing ([408c09d](https://github.com/mhatzl/mantra/commit/408c09dc01a470ecfd5439fadbf764f00b1e3716))
* ignore wiki-links in verbatim context ([cb1137b](https://github.com/mhatzl/mantra/commit/cb1137b3bd578db0e4f3a99b165f6de1fcd4a5a4))
* improve release report display for wiki links ([2de0e72](https://github.com/mhatzl/mantra/commit/2de0e72b18bcdc205d17727ba25a9b8fa0df52c3))
* keep spaces as en spaces for summary ([77482ee](https://github.com/mhatzl/mantra/commit/77482eebc6284042e67f4a4567d435e20e5eb1b8))
* make filepaths in tests os independent ([1be5a34](https://github.com/mhatzl/mantra/commit/1be5a344e457d5f58ee61b32c930b615ba4d5be8))
* mark output as env var for cat ([4941087](https://github.com/mhatzl/mantra/commit/49410873d12210905b8e45568f2fe97f763fba1d))
* only output stdout in check workflow ([5a03fa7](https://github.com/mhatzl/mantra/commit/5a03fa79c59d7d671d8546d166b3f4538d25c807))
* remove auto generated wiki-links ([59f51e4](https://github.com/mhatzl/mantra/commit/59f51e4a4bcef5de659d42b4aafd3e21ea18052b))
* remove auto-lock-pr action ([14b0d6e](https://github.com/mhatzl/mantra/commit/14b0d6e93e102798f146d8c61788e38a780b2444))
* remove clutter from templates ([67fe548](https://github.com/mhatzl/mantra/commit/67fe548123249941350e79924249cfde0a2077e9))
* remove dead code ([ddbd8c0](https://github.com/mhatzl/mantra/commit/ddbd8c0b91cf68b693df6b1c41ba68859e090931))
* remove git status from mantra action ([5486d74](https://github.com/mhatzl/mantra/commit/5486d74f5c58d31bd708a4c8a909462cd896c0d7))
* remove GITHUB_OUTPUT from workflow ([0ae9f64](https://github.com/mhatzl/mantra/commit/0ae9f6462ae5884086d558aa9febb17d83ed02eb))
* remove link cmd ([ef45e59](https://github.com/mhatzl/mantra/commit/ef45e5969e107bd7313f1b7bfc0c5bc074a67e55)), closes [#13](https://github.com/mhatzl/mantra/issues/13)
* remove manual flag TODO ([6b569c4](https://github.com/mhatzl/mantra/commit/6b569c4ec55d1a736825c60980dbb2c12b6e9135))
* remove quote char in shell replacement ([e1d3d4e](https://github.com/mhatzl/mantra/commit/e1d3d4e85374132f5b95ee370a2ce4d781d6b192))
* remove quotes in echo ([7550372](https://github.com/mhatzl/mantra/commit/7550372ac33f92dd4cc6814cf37e12881307fc61))
* remove summary prints in workflow ([19583c3](https://github.com/mhatzl/mantra/commit/19583c370a135635c5245be25d1f52b5b448dfce))
* remove unused container volume settings ([82ecd11](https://github.com/mhatzl/mantra/commit/82ecd110ff2929683d33d36891ca0f62f9ce45db))
* rename release report step ([5ac2585](https://github.com/mhatzl/mantra/commit/5ac25850999574152534d693a31a273a8cca7b44))
* replace bad req id with correct one ([7e4a5a5](https://github.com/mhatzl/mantra/commit/7e4a5a5fbb6564adbd1b0f711e067925ba45e3c7))
* replace escaped new line for summary ([cfdf2df](https://github.com/mhatzl/mantra/commit/cfdf2df70c31fd553607659a03d4d2481ead9de8))
* replace escaped newline in pr comment ([edfa7a0](https://github.com/mhatzl/mantra/commit/edfa7a0370bdee8a4e4b1df8e504026e9e2aa182))
* replace newline after quotes ([451cdc1](https://github.com/mhatzl/mantra/commit/451cdc1659ce499f95aac87592372e48b0ddb5b5))
* run mantra link explicitly in action ([bf7e495](https://github.com/mhatzl/mantra/commit/bf7e4956f97d434b854ad2b51acd4032a30cd505))
* set branch name automatically in sync-ci ([ea1a239](https://github.com/mhatzl/mantra/commit/ea1a239914c09dbbc352af0c4f5c1b106bebfb65))
* set wiki path directly to mantra cmd ([85007a4](https://github.com/mhatzl/mantra/commit/85007a4ff02c7cccdc67f7b0d39791bc9a0a67ed))
* surround output in quotes ([5c001cd](https://github.com/mhatzl/mantra/commit/5c001cdbf99c7aa23880ebf0854c7b2adf18236a))
* try adapt overview to accepted output format ([f887b3f](https://github.com/mhatzl/mantra/commit/f887b3fee8fe8d9e9519186aa83dc0447bc72f5c))
* update doc-comment ([e551e45](https://github.com/mhatzl/mantra/commit/e551e45110c19e757481f95c92ae0ecb6d0852d9))
* use *main* tag for docker image ([8bea695](https://github.com/mhatzl/mantra/commit/8bea695ce5be3caf63def1c212bc54a1f80e625e))
* use actual newline as replacement ([8b481ec](https://github.com/mhatzl/mantra/commit/8b481ecbf36d26d4ac55bffd3edd13e30a2d7720))
* use en-space to escape regular spaces ([7b110d3](https://github.com/mhatzl/mantra/commit/7b110d3bd7a1a2e129b8d9b34b1e1b35b89ab008))
* use github output env in workflow ([d9c61c3](https://github.com/mhatzl/mantra/commit/d9c61c338886aa933c43963afc9218482149d2f7))
* use PAT with workflow permissions ([6ad2653](https://github.com/mhatzl/mantra/commit/6ad26532782c5147e11a2a78d51d980f1d3abd0e))
* use relative paths on empty wiki-prefix ([b6bcfbc](https://github.com/mhatzl/mantra/commit/b6bcfbc5657586459795e271e924e9fc691abffd))
* use target branch name for pr check ([f1ea78b](https://github.com/mhatzl/mantra/commit/f1ea78b51a1f94069111c061650e48e2f27d5aa8))
* wrap output in echo ([33fde6b](https://github.com/mhatzl/mantra/commit/33fde6b923f12b24599a6fa2e9166ab2a50dd1e0))
* write overview to workflow summary ([33a2a19](https://github.com/mhatzl/mantra/commit/33a2a19b2f2d0b171eb31730444bb3c8fc2a6766))


### Architectur/Refactor

* add err logs in link cmd ([9e2eaa5](https://github.com/mhatzl/mantra/commit/9e2eaa5be4499010ac093d64dc79c70638ec8b48))
* move req module to wiki module ([e1ac1e4](https://github.com/mhatzl/mantra/commit/e1ac1e41ee5acfce32dee1696de9e96efeb76301))
* move src to sub folder ([a99d0f2](https://github.com/mhatzl/mantra/commit/a99d0f28c126296c2fc0eccdd3bb6fbde39a51ff))
* remove cargo workspace ([ba64134](https://github.com/mhatzl/mantra/commit/ba64134afe1c1d74eaf96263f15eb1c1f4f68cc0))
* remove volumes from container ([e8cdec4](https://github.com/mhatzl/mantra/commit/e8cdec4603edb6233962eb3e79df4c24fdd5e021))

## 0.1.0 (2023-09-01)


### Features

* add in-sync info ([219ec5c](https://github.com/mhatzl/mantra/commit/219ec5c837edb64833bece61fda704ccc166db74))
* add map of wiki files having new ref cnts ([91a4320](https://github.com/mhatzl/mantra/commit/91a432065e5c2afb46c90448e99b746772a36bfe))
* add references map ([00a3f90](https://github.com/mhatzl/mantra/commit/00a3f90e45ad01f114445804c18e450a39dbb5c5))
* add test to check for correct wiki-link ([ebfe163](https://github.com/mhatzl/mantra/commit/ebfe163139f6c267700bf1f9e786ec3926b081f2))
* add wiki-link cmd ([c4c1d6f](https://github.com/mhatzl/mantra/commit/c4c1d6f9c008f5d7410c49b2d72ab36b15531143))
* create req heading matcher ([530b453](https://github.com/mhatzl/mantra/commit/530b453f71a277ef3b72abeb063668ca8713dae3))
* create wiki structure from directory ([e629038](https://github.com/mhatzl/mantra/commit/e6290388d07ababd369005d5e42eb1e8fd1d18fc))
* detect reference changes ([d235358](https://github.com/mhatzl/mantra/commit/d235358b555248929039bf9030db773ca6973a2d))
* extract reqs from single file ([89fb8c9](https://github.com/mhatzl/mantra/commit/89fb8c92fcede25c1c92efe5301d1653a08846f8))
* impl basic sync functionality ([d80d948](https://github.com/mhatzl/mantra/commit/d80d9486044ef84df7af7ecdcf5e63ffdf0c5d21))
* impl wiki-link creation ([fedafb4](https://github.com/mhatzl/mantra/commit/fedafb4e4139bab667037a05a7d01826841f34a2))
* setup mantra as rust crate ([124f0c6](https://github.com/mhatzl/mantra/commit/124f0c61b80baae85b5d04d5f2621c55f46dd03c))
* setup sync as cli command ([cec408d](https://github.com/mhatzl/mantra/commit/cec408db56ce0fe8288dc80fc1b7dbe7619875f0))


### Bug Fixes

* adapt template specific content ([7421dcc](https://github.com/mhatzl/mantra/commit/7421dccd19acadc187bd036a43e222e1e172b2df))
* add link to wiki ([aafd55c](https://github.com/mhatzl/mantra/commit/aafd55ccb0e869380f3572cbfdb7bf4acdb841b9))
* apply clippy suggestions ([a9fc32b](https://github.com/mhatzl/mantra/commit/a9fc32bb279c7186fff7494f4c7b46612fed7d35))
* correct meaning of statement ([7b5431a](https://github.com/mhatzl/mantra/commit/7b5431a44d33c735b114fad1f7bf521a65874efa))
* correct spelling ([8e4c18e](https://github.com/mhatzl/mantra/commit/8e4c18efef1b8a3a4c236f9c0ef688112c365d89))
* correct spelling ([d7231bd](https://github.com/mhatzl/mantra/commit/d7231bd58f658db799dce3a9cd8b9189c2ed0b85))
* detect wrong links and end-of-file newline ([4207d51](https://github.com/mhatzl/mantra/commit/4207d5194a431ce5ff480a9601bc596e3cc4e969))
* determine cnt changes between wiki and RefMap ([4a4a4ae](https://github.com/mhatzl/mantra/commit/4a4a4aec4d0d733867f34209370c86a70c3bb822))
* enforce references list start ([838fa68](https://github.com/mhatzl/mantra/commit/838fa68dbe32cdb98e28dc75c3ed998fce0d4d59))
* fix req id used in test ([a7fcf21](https://github.com/mhatzl/mantra/commit/a7fcf219efee8a18a31635d576bc08bc5f4fd24d))
* improve cli help ([9f3ddbf](https://github.com/mhatzl/mantra/commit/9f3ddbfb01953db2ea32e8f86e3739b9006b2e8f))
* improve doc and req references ([209ee0b](https://github.com/mhatzl/mantra/commit/209ee0bc8909d98a09c599750b87df6c95ad44bb))
* remove dead code ([7e16a57](https://github.com/mhatzl/mantra/commit/7e16a57868024d04efcf604d2460c5f74506000f))
